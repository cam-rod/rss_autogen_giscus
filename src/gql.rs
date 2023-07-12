use std::sync::Arc;

use chrono::{Duration, Utc};
use cynic::http::CynicReqwestError;
use cynic::schema::QueryRoot;
use cynic::{http::ReqwestExt, GraphQlResponse, Operation, QueryFragment, QueryVariables};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tokio::task::spawn;

use crate::{HttpClients, Post};
use gh_gql_schema::{
    CategoryQuery, CategoryQueryVariables, CreateCommentsDiscussion,
    CreateCommentsDiscussionVariables, DiscussionExists, DiscussionExistsVariables, NullableString,
};

/// Executes a GraphQL call to the GitHub API.
pub async fn github_gql_query<T, Variables>(
    clients: Arc<HttpClients>,
    query_vars: Variables,
) -> Result<GraphQlResponse<T>, CynicReqwestError>
where
    Variables: QueryVariables + Serialize,
    T: QueryFragment<VariablesFields = Variables::Fields> + DeserializeOwned + 'static,
    T::SchemaType: QueryRoot,
{
    use cynic::QueryBuilder;

    let query: Operation<T, Variables> = T::build(query_vars);
    clients
        .gql
        .post(&clients.github_gql_url)
        .run_graphql(query)
        .await
}

/// Creates the GraphQL mutation to create a new discussion.
pub async fn create_graphql_request(
    clients: Arc<HttpClients>,
    post: Arc<Post>,
) -> Operation<CreateCommentsDiscussion, CreateCommentsDiscussionVariables> {
    use cynic::MutationBuilder;

    let repo_id = spawn(get_repo_id(Arc::clone(&clients)));
    let cat_id = spawn(get_category_id(Arc::clone(&clients)));

    // Append a description, if one was found.
    let mut full_desc = post.url.to_string();
    if let Some(mut post_desc) = post.description.clone() {
        post_desc.push_str("\n\n");
        full_desc.insert_str(0, post_desc.as_str());
    }

    CreateCommentsDiscussion::build(CreateCommentsDiscussionVariables {
        repo_id: repo_id.await.unwrap().unwrap(),
        cat_id: cat_id.await.unwrap().unwrap(),
        desc: full_desc,
        title: post.url.path().to_string(),
    })
}

/// Retrieves the numeric ID of the repo.
async fn get_repo_id(clients: Arc<HttpClients>) -> reqwest::Result<cynic::Id> {
    let repo_resp: Value = clients
        .gql
        .get(format!(
            "{}/repos/{}/{}",
            clients.github_rest_url, clients.repo_owner, clients.repo_name
        ))
        .send()
        .await?
        .json()
        .await?;
    Ok(repo_resp["id"].as_str().unwrap().into())
}

/// Retrieves the numeric ID of the discussion category.
async fn get_category_id(clients: Arc<HttpClients>) -> Result<cynic::Id, CynicReqwestError> {
    let mut page_end_cursor = NullableString::default();
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
                Some(matching_cat) => return Ok(matching_cat.name.clone().into()),
                None => {
                    // Check if there's another page of results
                    if categories.page_info.has_next_page {
                        page_end_cursor = categories.page_info.end_cursor.into();
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
) -> Result<Option<String>, CynicReqwestError> {
    let current_time = Utc::now();
    let max_lookback = Duration::days(7);

    let mut page_end_cursor = NullableString::default();
    loop {
        let discussion_exists_resp: GraphQlResponse<DiscussionExists> = github_gql_query(
            Arc::clone(&clients),
            DiscussionExistsVariables {
                owner: &clients.repo_owner,
                repo_name: &clients.repo_name,
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
                    // Don't check for discussions older than 7 days
                    if discussion
                        .created_at
                        .0
                        .parse::<chrono::DateTime<Utc>>()
                        .unwrap()
                        - current_time
                        > max_lookback
                    {
                        return Ok(None);
                    } else if Some(&discussion.title) == post.title.as_ref() {
                        return Ok(Some(discussion.url.0.clone()));
                    }
                }

                // Check if there's another page of results
                if discussions.page_info.has_next_page {
                    page_end_cursor = discussions.page_info.end_cursor.into();
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
