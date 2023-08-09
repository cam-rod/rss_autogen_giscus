#[cynic::schema("github")]
mod schema {}

// Base Types

#[derive(cynic::Scalar, Debug, Clone)]
pub struct DateTime(pub String);

#[derive(cynic::Scalar, Debug, Clone)]
#[cynic(graphql_type = "URI")]
pub struct Uri(pub String);

#[derive(cynic::QueryFragment, Debug)]
pub struct Discussion {
    pub id: cynic::Id,
    pub title: String,
    pub created_at: DateTime,
    pub url: Uri,
    pub body_text: String,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct PageInfo {
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
}

// query RepoIdQuery

#[derive(cynic::QueryVariables, Debug, Clone)]
pub struct RepoIdQueryVariables<'a> {
    pub owner: &'a str,
    pub repo_name: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "RepoIdQueryVariables")]
pub struct RepoIdQuery {
    #[arguments(owner: $owner, name: $repo_name)]
    pub repository: Option<RepoIdQueryRepository>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Repository")]
pub struct RepoIdQueryRepository {
    pub id: cynic::Id,
}

// query CategoryQuery

#[derive(cynic::QueryVariables, Debug, Clone)]
pub struct CategoryQueryVariables<'a> {
    pub owner: &'a str,
    pub repo_name: &'a str,
    pub after_cursor: Option<String>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "CategoryQueryVariables")]
pub struct CategoryQuery {
    #[arguments(owner: $owner, name: $repo_name)]
    pub repository: Option<CategoryQueryRepository>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Repository", variables = "CategoryQueryVariables")]
pub struct CategoryQueryRepository {
    #[arguments(first: 50, after: $after_cursor)]
    pub discussion_categories: DiscussionCategoryConnection,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct DiscussionCategoryConnection {
    #[cynic(flatten)]
    pub edges: Vec<DiscussionCategoryEdge>,
    pub page_info: PageInfo,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct DiscussionCategoryEdge {
    pub node: Option<DiscussionCategory>,
    pub cursor: String,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct DiscussionCategory {
    pub id: cynic::Id,
    pub name: String,
}

// query DiscussionExists

#[derive(cynic::QueryVariables, Debug, Clone)]
pub struct DiscussionExistsVariables<'a> {
    pub owner: &'a str,
    pub repo_name: &'a str,
    pub cat_id: cynic::Id,
    pub after_cursor: Option<String>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "DiscussionExistsVariables")]
pub struct DiscussionExists {
    #[arguments(owner: $owner, name: $repo_name)]
    pub repository: Option<DiscussionExistsRepository>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Repository", variables = "DiscussionExistsVariables")]
pub struct DiscussionExistsRepository {
    #[arguments(orderBy: { direction: "DESC", field: "CREATED_AT" }, categoryId: $cat_id, first: 50, after: $after_cursor)]
    pub discussions: DiscussionConnection,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct DiscussionConnection {
    #[cynic(flatten)]
    pub edges: Vec<DiscussionEdge>,
    pub page_info: PageInfo,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct DiscussionEdge {
    pub node: Option<Discussion>,
    pub cursor: String,
}

// mutation CreateCommentsDiscussion

#[derive(cynic::QueryVariables, Debug)]
pub struct CreateCommentsDiscussionVariables {
    pub repo_id: cynic::Id,
    pub cat_id: cynic::Id,
    pub desc: String,
    pub title: String,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    graphql_type = "Mutation",
    variables = "CreateCommentsDiscussionVariables"
)]
pub struct CreateCommentsDiscussion {
    #[arguments(input: { clientMutationId: "rss_autogen_giscus", body: $desc, categoryId: $cat_id, repositoryId: $repo_id, title: $title })]
    pub create_discussion: Option<CreateDiscussionPayload>,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct CreateDiscussionPayload {
    pub discussion: Option<Discussion>,
}

impl From<Uri> for String {
    fn from(value: Uri) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use cynic::Id;

    #[allow(unused_imports)]
    use super::schema;

    const REPO_OWNER: &str = "team-role-org-testing";
    const REPO_NAME: &str = "team-role-org-testing.github.io";

    #[test]
    fn repo_id_query_output() {
        use super::{RepoIdQuery, RepoIdQueryVariables};
        use cynic::QueryBuilder;

        let repo_id_query_op = RepoIdQuery::build(RepoIdQueryVariables {
            owner: REPO_OWNER,
            repo_name: REPO_NAME,
        });
        print!("{}", repo_id_query_op.query);
    }

    #[test]
    fn category_query_output() {
        use super::{CategoryQuery, CategoryQueryVariables};
        use cynic::QueryBuilder;

        let category_query_op = CategoryQuery::build(CategoryQueryVariables {
            owner: REPO_OWNER,
            repo_name: REPO_NAME,
            after_cursor: None,
        });
        print!("{}", category_query_op.query);
    }

    #[test]
    fn discussion_exists_output() {
        use super::{DiscussionExists, DiscussionExistsVariables};
        use cynic::QueryBuilder;

        let discussion_exists_op = DiscussionExists::build(DiscussionExistsVariables {
            owner: REPO_OWNER,
            repo_name: REPO_NAME,
            cat_id: Id::new("155"),
            after_cursor: None,
        });
        print!("{}", discussion_exists_op.query);
    }

    #[test]
    fn create_comments_discussion_output() {
        use super::{CreateCommentsDiscussion, CreateCommentsDiscussionVariables};
        use cynic::MutationBuilder;

        let create_comments_discussion_op =
            CreateCommentsDiscussion::build(CreateCommentsDiscussionVariables {
                cat_id: "DIC_kwDOJSVgjc4CVgpt".into(),
                desc: "Here is the description of a future post".to_string(),
                title: "/blog/most-recent-post.txt".to_string(),
                repo_id: "623206541".into(),
            });

        print!("{}", create_comments_discussion_op.query);
    }
}
