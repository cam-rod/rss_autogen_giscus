//! Autogenerates GitHub Discussions to be used by Giscus.
//!
//! This came out of a preference for a way to use Giscus, without requiring users to authenticate
//! with the app. Since the discussion isn't created until someone comments, we needed a way to
//! automatically create it once a blog post was uploaded.
//!
//! This crate checks for the latest post via the RSS feed, and then extracts the contents needed to
//! to create a post, formatted as follows:
//!
//! - **Title**: URL path of the post (not including base URL)
//! - **Description**: (potentially) First paragraph of the post, followed by a full link
//!
//! This crate works best when run as a GitHub Action, triggered by the completion of the
//! `pages-build-deployment` action for GitHub pages. It depends on the RSS feed being up-to-date at the time
//! of running, so you may need to introduce a delay.

mod constants;
mod gql;
mod post;

use std::time::Duration;

use octocrab::Octocrab;

use gql::create_graphql_request;
use post::{latest_post, post_description};

#[tokio::main]
pub async fn main() {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env var is required");
    let octocrab = Octocrab::builder()
        .personal_token(token)
        .build()
        .expect("Unable to build GitHub API client");

    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("Unable to build HTTP client");

    let post_url = latest_post(&client).await.unwrap();
    let post_desc = post_description(&client, post_url.as_str()).await.unwrap();
    let request = create_graphql_request(&octocrab, &post_url, &post_desc)
        .await
        .unwrap();

    let response: serde_json::Value = octocrab.graphql(&request).await.unwrap();
    if let Some(discussion_info) = response.get("data") {
        if discussion_info["id"].is_number()
            && discussion_info["title"].as_str().unwrap() == post_url.path()
        {
            println!(
                "Successfully created new discussion at {} ({})",
                discussion_info["url"].as_str().unwrap(),
                discussion_info["title"].as_str().unwrap()
            )
        }
    }

    panic!(
        "Dicussion could not be generated. GraphQL response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );
}
