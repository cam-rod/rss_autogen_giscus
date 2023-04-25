use octocrab::Octocrab;
use url::Url;

use crate::constants::{CATEGORY_NAME, REPO_NAME, REPO_OWNER};

pub async fn create_graphql_request(
    octocrab: &Octocrab,
    url: &Url,
    description: &String,
) -> octocrab::Result<String> {
    let repo_id = octocrab.repos(REPO_OWNER, REPO_NAME).get().await?.id;
    let categories: serde_json::Value = octocrab.graphql(&get_categories_gql()).await?;

    let mut category_id = "";
    for category in categories["data"]["repository"]["edges"]
        .as_array()
        .unwrap()
    {
        if let Some(id) = category["node"]["name"].as_str() {
            if id == CATEGORY_NAME {
                category_id = id;
                break;
            }
        }
    }
    if category_id.is_empty() {
        panic!("Category {CATEGORY_NAME} was not present in repository {REPO_OWNER}/{REPO_NAME}")
    }

    Ok(create_discussion_gql(&repo_id.to_string(), category_id, url, description))
}

/// GraphQL string generators

fn get_categories_gql() -> String {
    format!(
        "\
query {{
  repository(owner: \"{}\", name: \"{}\") {{
    discussionCategories() {{
      edges {{
        node {{
          name
          id
        }}
      }}
    }}
  }}
}}",
        REPO_OWNER, REPO_NAME
    )
}

fn create_discussion_gql(repo: &str, category: &str, url: &Url, description: &String) -> String {
    format!("\
mutation {{
  createDiscussion(input: {{repositoryId: {repo}, categoryId: {category}, title: \"{}\", body: \"{description}\") {{
    discussion {{
      id
      title
      url
    }}
  }}
}}", url.path())
}
