use std::sync::Arc;
use std::time::Duration;

use cynic::http::CynicReqwestError;
use cynic::schema::QueryRoot;
use cynic::{http::ReqwestExt, GraphQlResponse, Id, Operation, QueryFragment, QueryVariables};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::time::sleep;

use crate::{HttpClients, Post};
use gh_gql_schema::{
    CategoryQuery, CategoryQueryVariables, CreateCommentsDiscussion,
    CreateCommentsDiscussionVariables, DiscussionExists, DiscussionExistsVariables, RepoIdQuery,
    RepoIdQueryVariables,
};

/// Executes a GraphQL call to the GitHub API, respecting rate limits.
///
/// To support rate limits, `query_vars` must also implement [`Clone`].
pub async fn github_gql_query<T, Variables>(
    clients: Arc<HttpClients>,
    query_vars: Variables,
) -> Result<GraphQlResponse<T>, CynicReqwestError>
where
    Variables: QueryVariables + Serialize + Clone,
    T: QueryFragment<VariablesFields = Variables::Fields> + DeserializeOwned + 'static,
    T::SchemaType: QueryRoot,
{
    use cynic::QueryBuilder;

    let query_attempts = vec![query_vars.clone(); 5];
    let mut attempt = 0;
    for vars in query_attempts {
        attempt += 1;
        let resp = clients
            .gql
            .post(&clients.github_gql_url)
            .run_graphql(T::build(vars))
            .await;

        if let Err(CynicReqwestError::ErrorResponse(err_status, err_body)) = resp {
            if err_body.contains("Server Error") {
                gql_sleep(err_status, err_body, 30).await;
                continue;
            }
            match err_status.as_u16() {
                403 => {
                    gql_sleep(err_status, err_body, 5 * (2_u64).pow(attempt)).await;
                    continue;
                } // Rate limit reached
                401 => panic!("Invalid authentication tokens:\n{:#?}", clients.gql),
                400..=599 => {
                    gql_sleep(err_status, err_body, 30).await;
                    continue;
                }
                300..=399 => panic!(
                    "Unexpected redirection response ({}): {}",
                    err_status, err_body
                ),
                _ => panic!("Unhandled HTTP status code ({}): {}", err_status, err_body),
            }
        } else {
            return resp;
        }
    }

    panic!(
        "Exceeded maximum of 5 attempts while executing {}",
        T::build(query_vars).operation_name.unwrap()
    );
}

/// Sleep for a period of time upon receiving a non-200 status code from [`github_gql_query`].
async fn gql_sleep(status: StatusCode, body: String, sleep_secs: u64) {
    eprintln!(
        "Request failed ({}): {}\nSleeping for {} seconds...",
        status, body, sleep_secs
    );
    sleep(Duration::from_secs(sleep_secs)).await;
}

/// Creates the GraphQL mutation to create a new discussion.
pub async fn create_graphql_request(
    clients: Arc<HttpClients>,
    post: Arc<Post>,
    cat_id: Arc<Id>,
) -> Operation<CreateCommentsDiscussion, CreateCommentsDiscussionVariables> {
    use cynic::MutationBuilder;

    let repo_id = get_repo_id(Arc::clone(&clients)).await;

    // Append a description, if one was found.
    let mut full_desc = post.url.to_string();
    if let Some(mut post_desc) = post.description.clone() {
        post_desc.push_str("\n\n");
        full_desc.insert_str(0, post_desc.as_str());
    }

    CreateCommentsDiscussion::build(CreateCommentsDiscussionVariables {
        repo_id: repo_id.unwrap(),
        cat_id: cat_id.as_ref().clone(),
        desc: full_desc,
        title: post.url.path().to_string(),
    })
}

/// Retrieves the numeric ID of the repo.
async fn get_repo_id(clients: Arc<HttpClients>) -> Result<Id, CynicReqwestError> {
    let repo_resp: GraphQlResponse<RepoIdQuery> = github_gql_query(
        Arc::clone(&clients),
        RepoIdQueryVariables {
            owner: &clients.repo_owner,
            repo_name: &clients.repo_name,
        },
    )
    .await?;

    if let Some(repo_id) = repo_resp
        .data
        .and_then(|d| d.repository)
        .map(|repo| repo.id)
    {
        Ok(repo_id)
    } else {
        panic!(
            "Repo ID could not be retrieved. GraphQL errors:\n{:#?}",
            repo_resp.errors.unwrap()
        );
    }
}

/// Retrieves the numeric ID of the discussion category.
pub async fn get_category_id(clients: Arc<HttpClients>) -> Result<Id, CynicReqwestError> {
    let mut page_end_cursor = None;
    loop {
        let category_resp: GraphQlResponse<CategoryQuery> = github_gql_query(
            Arc::clone(&clients),
            CategoryQueryVariables {
                owner: &clients.repo_owner,
                repo_name: &clients.repo_name,
                after_cursor: page_end_cursor,
            },
        )
        .await?;

        if let Some(categories) = category_resp
            .data
            .and_then(|d| d.repository)
            .map(|repo| repo.discussion_categories)
        {
            match categories
                .edges
                .iter()
                .flat_map(|c| &c.node)
                .find(|cat| cat.name == clients.discussion_category)
            {
                Some(matching_cat) => return Ok(matching_cat.id.clone()),
                None => {
                    // Check if there's another page of results
                    if categories.page_info.has_next_page {
                        page_end_cursor = categories.page_info.end_cursor;
                        continue;
                    } else {
                        panic!(
                            "Category {} was not present in repository {}/{}",
                            clients.discussion_category, clients.repo_owner, clients.repo_name
                        );
                    }
                }
            }
        } else {
            panic!(
                "No discussion categories found! GraphQL errors:\n{:#?}",
                category_resp.errors.unwrap()
            );
        }
    }
}

/// Checks if a discussion with the same title already exists, before creating a new one.
pub async fn discussion_exists(
    clients: Arc<HttpClients>,
    post: Arc<Post>,
    cat_id: Arc<Id>,
) -> Result<Option<String>, CynicReqwestError> {
    let current_time = chrono::Utc::now();
    let max_lookback = chrono::Duration::days(clients.lookback_days);

    let mut page_end_cursor = None;
    loop {
        if cfg!(test) {
            println!("Current after_cursor is {:?}", &page_end_cursor);
        }

        let discussion_exists_resp: GraphQlResponse<DiscussionExists> = github_gql_query(
            Arc::clone(&clients),
            DiscussionExistsVariables {
                owner: &clients.repo_owner,
                repo_name: &clients.repo_name,
                cat_id: cat_id.as_ref().clone(),
                after_cursor: page_end_cursor,
            },
        )
        .await?;

        if discussion_exists_resp.errors.is_none() {
            if let Some(discussions) = discussion_exists_resp
                .data
                .and_then(|data| data.repository)
                .map(|repo| repo.discussions)
            {
                for discussion in discussions
                    .edges
                    .iter()
                    .filter_map(|edge| edge.node.as_ref())
                {
                    // Don't check for discussions older than the lookback period, if enabled
                    if !max_lookback.is_zero()
                        && current_time
                            - discussion
                                .created_at
                                .0
                                .parse::<chrono::DateTime<chrono::Utc>>()
                                .unwrap()
                            > max_lookback
                    {
                        return Ok(None);
                    } else if post.url.path().contains(&discussion.title) {
                        // Giscus strips the leading stash and file extension from the URL when posting, but still recognizes it.
                        return Ok(Some(discussion.url.0.clone()));
                    }
                }

                // Check if there's another page of results
                if discussions.page_info.has_next_page {
                    page_end_cursor = discussions.page_info.end_cursor;
                    continue;
                } else {
                    return Ok(None);
                }
            }
        }

        panic!(
            "Unable to query existing repos. GraphQL errors: \n{:#?}",
            discussion_exists_resp.errors
        );
    }
}

#[cfg(test)]
mod tests {
    //! You must set the `GITHUB_TOKEN` environment variable to run these tests.
    //!
    //! **Note:** these tests operate on the live GitHub API, so be mindful of any potential rate limiting
    use std::sync::Arc;

    use cynic::Id;
    use serial_test::serial;
    use tokio_test::assert_ok;
    use url::Url;

    use crate::gql::{create_graphql_request, discussion_exists, get_category_id, get_repo_id};
    use crate::{HttpClients, Post};

    const BLOG_CATEGORY_ID: &str = "DIC_kwDOJSVgjc4CVgpt";
    const QA_CATEGORY_ID: &str = "DIC_kwDOJSVgjc4CVgpd";
    const TEST_REPO_ID: &str = "R_kgDOJSVgjQ";

    #[tokio::test]
    #[serial]
    async fn test_blogs_category_query() {
        let clients = Arc::new(HttpClients::test_setup(false));
        let category_id = get_category_id(clients).await;

        assert_ok!(&category_id);
        assert_eq!(category_id.unwrap(), Id::new(BLOG_CATEGORY_ID))
    }

    #[tokio::test]
    #[serial]
    async fn test_qa_category_query() {
        let clients = Arc::new(HttpClients {
            discussion_category: "Q&A".to_string(),
            ..HttpClients::test_setup(false)
        });
        let category_id = get_category_id(clients).await;

        assert_ok!(&category_id);
        assert_eq!(category_id.unwrap(), Id::new(QA_CATEGORY_ID))
    }

    #[tokio::test]
    #[serial]
    #[should_panic]
    async fn test_missing_category_query() {
        let clients = Arc::new(HttpClients {
            discussion_category: "Removed".to_string(),
            ..HttpClients::test_setup(false)
        });
        let category_id = get_category_id(clients).await;
        assert_ok!(&category_id);
    }

    #[tokio::test]
    #[serial]
    async fn test_get_repo_id() {
        let clients = Arc::new(HttpClients::test_setup(false));
        let repo_id = get_repo_id(clients).await;
        assert_ok!(&repo_id);
        assert_eq!(repo_id.unwrap(), Id::new(TEST_REPO_ID));
    }

    #[tokio::test]
    #[serial]
    async fn test_discussion_exists() {
        let clients = Arc::new(HttpClients {
            lookback_days: 0,
            ..HttpClients::test_setup(false)
        });
        let post = Arc::new(Post {
            description: Some("Doesn't matter".to_string()),
            url: Url::parse("https://team-role-org-testing.github.io/jekyll/update/2023/04/03/welcome-to-jekyll.html").unwrap(),
        });

        let prev_discussion = discussion_exists(
            clients,
            post,
            Arc::new(Id::new(BLOG_CATEGORY_ID.to_string())),
        )
        .await;
        assert_ok!(&prev_discussion);
        assert_eq!(prev_discussion.unwrap(), Some("https://github.com/team-role-org-testing/team-role-org-testing.github.io/discussions/1".to_string()));
    }

    #[tokio::test]
    #[serial]
    async fn test_discussion_not_exists() {
        let clients = Arc::new(HttpClients {
            lookback_days: 0,
            ..HttpClients::test_setup(false)
        });
        let post = Arc::new(Post {
            description: None,
            url: Url::parse("https://www.cbc.ca").unwrap(),
        });

        let prev_discussion = discussion_exists(
            clients,
            post,
            Arc::new(Id::new(BLOG_CATEGORY_ID.to_string())),
        )
        .await;
        assert_ok!(&prev_discussion);
        assert_eq!(prev_discussion.unwrap(), None);
    }

    /// Testing is done on the orgs/community _(internally, `community/community`)_ repo, a relatively active instance
    #[tokio::test]
    #[serial]
    async fn test_discussion_paging() {
        let community_general_cat_id = Id::new("DIC_kwDOEfmk4M4B92AT".to_string());

        let clients = Arc::new(HttpClients {
            repo_owner: "community".to_string(),
            repo_name: "community".to_string(),
            discussion_category: "General".to_string(),
            lookback_days: 15,
            ..HttpClients::test_setup(false)
        });
        let post = Arc::new(Post {
            description: None,
            url: Url::parse(
                "irc://a.completely.gibberish.url.that.would.never.be.found/123jf9a92k",
            )
            .unwrap(),
        });
        assert_eq!(
            get_category_id(Arc::clone(&clients)).await.unwrap(),
            community_general_cat_id
        );

        let existing_discussion =
            discussion_exists(clients, post, Arc::new(community_general_cat_id)).await;
        assert_ok!(&existing_discussion);
        assert_eq!(existing_discussion.unwrap(), None);
    }

    #[tokio::test]
    #[serial]
    async fn test_generate_mutation() {
        let clients = Arc::new(HttpClients::test_setup(false));
        let post = Post::get_latest(&clients).await.unwrap();
        let cat_id = get_category_id(Arc::clone(&clients)).await.unwrap();

        let mutation = create_graphql_request(
            Arc::clone(&clients),
            Arc::clone(&post),
            Arc::new(cat_id.clone()),
        )
        .await;

        assert_eq!(mutation.variables.cat_id, cat_id);
        assert_eq!(mutation.variables.title, post.url.path());
        assert_eq!(mutation.variables.repo_id, Id::new(TEST_REPO_ID));
        assert!(&mutation.variables.desc.contains(post.url.as_str()));
        assert!(&mutation
            .variables
            .desc
            .contains(post.description.as_ref().unwrap().as_str()));
        println!(
            "Operation: {}\n\nQuery:\n{}\n\nVariables:\n{:#?}",
            &mutation
                .operation_name
                .expect("Mutation operation name not found"),
            mutation.query,
            &mutation.variables
        );
    }
}
