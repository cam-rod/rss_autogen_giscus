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
}

#[derive(cynic::QueryFragment, Debug)]
pub struct PageInfo {
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
}

// query DiscussionExists

#[derive(cynic::QueryVariables, Debug)]
pub struct DiscussionExistsVariables<'a> {
    pub owner: &'a str,
    pub repo_name: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "DiscussionExistsVariables")]
pub struct DiscussionExists {
    #[arguments(owner: $owner, name: $repo_name)]
    pub repository: Option<DiscussionExistsRepository>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Repository")]
pub struct DiscussionExistsRepository {
    #[arguments(orderBy: { direction: "DESC", field: "CREATED_AT" }, first: 50)]
    pub discussions: DiscussionConnection,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct DiscussionConnection {
    pub edges: Option<Vec<Option<DiscussionEdge>>>,
    pub page_info: PageInfo,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct DiscussionEdge {
    pub node: Option<Discussion>,
    pub cursor: String,
}

// query CategoryQuery

#[derive(cynic::QueryVariables, Debug)]
pub struct CategoryQueryVariables<'a> {
    pub owner: &'a str,
    pub repo_name: &'a str,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Query", variables = "CategoryQueryVariables")]
pub struct CategoryQuery {
    #[arguments(owner: $owner, name: $repo_name)]
    pub repository: Option<CategoryQueryRepository>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(graphql_type = "Repository")]
pub struct CategoryQueryRepository {
    #[arguments(first: 50)]
    pub discussion_categories: DiscussionCategoryConnection,
}

#[derive(cynic::QueryFragment, Debug)]
pub struct DiscussionCategoryConnection {
    pub edges: Option<Vec<Option<DiscussionCategoryEdge>>>,
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

// mutation CreateCommentsDiscussion

#[derive(cynic::QueryVariables, Debug)]
pub struct CreateCommentsDiscussionVariables {
    pub repo_id: cynic::Id,
    pub cat_id: cynic::Id,
    pub desc: String,
    pub post_rel_path: String,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    graphql_type = "Mutation",
    variables = "CreateCommentsDiscussionVariables"
)]
pub struct CreateCommentsDiscussion {
    #[arguments(input: { body: $desc, categoryId: $cat_id, repositoryId: $repo_id, title: $post_rel_path })]
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
    use super::*;

    const REPO_OWNER: &str = "team-role-org-testing";
    const REPO_NAME: &str = "team-role-org-testing.github.io";

    #[test]
    fn discussion_exists_output() {
        use cynic::QueryBuilder;

        let discussion_exists_op = DiscussionExists::build(DiscussionExistsVariables {
            owner: REPO_OWNER,
            repo_name: REPO_NAME,
        });
        print!("{}", discussion_exists_op.query);
    }

    #[test]
    fn category_query_output() {
        use cynic::QueryBuilder;

        let category_query_op = CategoryQuery::build(CategoryQueryVariables {
            owner: REPO_OWNER,
            repo_name: REPO_NAME,
        });
        print!("{}", category_query_op.query);
    }

    #[test]
    fn create_comments_discussion_output() {
        use cynic::MutationBuilder;

        let create_comments_discussion_op =
            CreateCommentsDiscussion::build(CreateCommentsDiscussionVariables {
                cat_id: "DIC_kwDOJSVgjc4CVgpt".into(),
                desc: "Here is the description of a future post".to_string(),
                post_rel_path: "/blog/most-recent-post.txt".to_string(),
                repo_id: "623206541".into(),
            });

        print!("{}", create_comments_discussion_op.query);
    }
}
