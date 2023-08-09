//! Autogenerate GitHub Discussions to be used by Giscus.
//!
//! This came from a need to support Giscus without requiring users to authenticate
//! with the app. Since the discussion isn't created until someone comments, we needed a way to
//! automatically create it once a blog post was uploaded.
//!
//! This crate checks for the latest post in the blog's RSS feed, and then extracts the contents
//! needed to create a discussion post, formatted as follows:
//!
//! - **Title**: URL path of the post (not including base URL)
//! - **Description**: Pulled from the `<meta name="description">` tag, followed by a full link
//!
//! The program works best when run after the RSS feed has been updated with the most recent post.
//! This may require you to introduce a delay.

mod gql;
mod post;

use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use cynic::http::{CynicReqwestError, ReqwestExt};
use reqwest::header::USER_AGENT;
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION},
    Client,
};
use tokio::join;

pub use post::Post;

use gql::{create_graphql_request, discussion_exists, get_category_id};

/// Monostruct containing the HTML and GraphQL clients used to create the discussion, along with the
/// necessary URLs.
#[derive(Debug, Clone)]
pub struct HttpClients {
    /// HTML client for accessing the RSS feed, blog post, and GitHub REST API.
    pub html: Client,

    /// GraphQL client for accessing the GitHub GraphQL API. This client must be created with the
    /// following headers, using [`ClientBuilder::default_headers`](reqwest::ClientBuilder::default_headers):
    ///
    /// - `Accept: application/vnd.github+json`
    /// - `Authorization: <GitHub token>`
    /// - `User-Agent: <appropriate user agent name>`
    /// - `X-Github-Next-Global-ID: 1`
    pub gql: Client,

    /// URL for the blog's RSS feed.
    pub website_rss_url: String,

    /// URL for GitHub REST API, typically <https://api.github.com>
    pub github_rest_url: String,

    /// URL for GitHub GraphQL API, typically <https://api.github.com/graphql>
    pub github_gql_url: String,

    /// Owner of the repository hosting the comments.
    pub repo_owner: String,

    /// Name of the repository hosting the comments
    pub repo_name: String,

    /// Name of the discussion category that the comments should be posted under.
    pub discussion_category: String,

    /// The number of days to look back in history, to check if a previous discussion occurred.
    /// Limit is disabled if set to 0.
    pub lookback_days: i64,
}

impl HttpClients {
    /// Create the reqwest clients, and pull the other values from environment variables. These are
    /// assumed to be formatted like the
    /// [default values available in GitHub Actions](https://docs.github.com/en/actions/learn-github-actions/variables#default-environment-variables),
    /// except for `DISCUSSION_CATEGORY` and `LOOKBACK_DAYS`:
    ///
    /// - `GITHUB_TOKEN`, used in the authorization header for the [GraphQL client](HttpClients::gql)
    /// - [`WEBSITE_RSS_URL`](HttpClients::website_rss_url), required
    /// - [`GITHUB_API_URL`](HttpClients::github_rest_url), optional (defaults to <https://api.github.com>)
    /// - [`GITHUB_GRAPHQL_URL`](HttpClients::github_gql_url), optional (defaults to <https://api.github.com/graphql>)
    /// - [`GITHUB_REPOSITORY_OWNER`](HttpClients::repo_owner), required
    /// - `GITHUB_REPOSITORY` in format `<owner>/<repo>`, required (mapped to [`repo_name`](HttpClients::repo_name))
    /// - [`DISCUSSION_CATEGORY`](HttpClients::discussion_category) as the name of the category to post under, required
    /// - [`LOOKBACK_DAYS`](HttpClients::lookback_days), optional (defaults to 7)
    ///
    /// ```rust
    /// use std::env;
    /// use rss_autogen_giscus::HttpClients;
    ///
    /// env::set_var("WEBSITE_RSS_URL", "https://rss.cbc.ca/lineup/topstories.xml");
    /// env::set_var("GITHUB_TOKEN", "secret_github_pat");
    /// env::set_var("GITHUB_REPOSITORY_OWNER", "microsoft");
    /// env::set_var("GITHUB_REPOSITORY", "microsoft/vscode");
    /// env::set_var("DISCUSSION_CATEGORY", "CBC News");
    /// env::set_var("LOOKBACK_DAYS", "0");
    ///
    /// let clients = HttpClients::init();
    /// ```
    pub fn init() -> Arc<Self> {
        let (html_client, gql_client) = Self::clients(false);

        Arc::new(Self {
            html: html_client,
            gql: gql_client,
            website_rss_url: env::var("WEBSITE_RSS_URL")
                .expect("WEBSITE_RSS_URL env var is required"),

            github_rest_url: env::var("GITHUB_API_URl")
                .unwrap_or("https://api.github.com".to_string()),
            github_gql_url: env::var("GITHUB_GRAPHQL_URL")
                .unwrap_or("https://api.github.com/graphql".to_string()),
            repo_owner: env::var("GITHUB_REPOSITORY_OWNER")
                .expect("Repo owner was not found (GITHUB_REPOSITORY_OWNER)"),
            repo_name: env::var("GITHUB_REPOSITORY")
                .unwrap()
                .split_once('/')
                .expect("Not a valid repo/name string")
                .1
                .into(),
            discussion_category: env::var("DISCUSSION_CATEGORY")
                .expect("DISCUSSION_CATEGORY env var is required"),
            lookback_days: env::var("LOOKBACK_DAYS")
                .map_or(7, |e| i64::from_str(e.as_str()).unwrap()),
        })
    }

    /// A small function to create the HTML and GraphQL clients, mainly for testing purposes.
    ///
    /// Passing `true` will replace `GITHUB_TOKEN` with a fake value, so that the environment
    /// variable does not need to be set.
    fn clients(use_placeholder_github_token: bool) -> (Client, Client) {
        let token = match use_placeholder_github_token {
            true => String::from("00112233FAKE_TOKEN44556677"),
            false => env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env var is required"),
        };

        let mut gh_headers = HeaderMap::new();
        gh_headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(format!("Bearer {token}").as_str()).unwrap(),
        );
        gh_headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        gh_headers.insert(
            "X-Github-Next-Global-ID",
            HeaderValue::from_str("1").unwrap(),
        );
        gh_headers.insert(
            USER_AGENT,
            HeaderValue::from_str("rss_autogen_giscus").unwrap(),
        );

        (
            Client::builder()
                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) rss-autogen-giscus/0.1.0 Chrome/113.0.0.0 Safari/537.36")
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Unable to build REST client"),
            Client::builder()
                .timeout(Duration::from_secs(60))
                .default_headers(gh_headers)
                .build()
                .expect("Unable to build GraphQL client")
            )
    }

    /// Creates an instance for testing purposes.
    ///
    /// If the GITHUB_TOKEN does not need to be set, a placeholder value can be used.
    #[cfg(test)]
    fn test_setup(use_placeholder_github_token: bool) -> Self {
        let (html, gql) = Self::clients(use_placeholder_github_token);
        Self {
            html,
            gql,
            website_rss_url: "https://team-role-org-testing.github.io/feed.xml".to_string(),
            github_rest_url: "https://api.github.com".to_string(),
            github_gql_url: "https://api.github.com/graphql".to_string(),
            repo_owner: "team-role-org-testing".to_string(),
            repo_name: "team-role-org-testing.github.io".to_string(),
            discussion_category: "Blogs".to_string(),
            lookback_days: 7,
        }
    }
}

/// Create the GitHub Discussion post for Giscus.
///
/// ```rust
/// use cynic::http::CynicReqwestError;
/// use rss_autogen_giscus::{create_discussion, HttpClients, Post};
///
/// #[tokio::main]
/// pub async fn main() -> Result<(), CynicReqwestError> {
///     let clients = HttpClients::init();
///     let latest_post = Post::get_latest(&clients).await?;
///
///     create_discussion(clients, latest_post).await
/// }
/// ```
pub async fn create_discussion(
    clients: Arc<HttpClients>,
    post: Arc<Post>,
) -> Result<(), CynicReqwestError> {
    let cat_id = Arc::new(get_category_id(Arc::clone(&clients)).await?);

    let (is_existing_discussion, create_disc_op) = join!(
        discussion_exists(Arc::clone(&clients), Arc::clone(&post), Arc::clone(&cat_id)),
        create_graphql_request(Arc::clone(&clients), Arc::clone(&post), Arc::clone(&cat_id))
    );

    if is_existing_discussion.as_ref().unwrap().is_some() {
        println!(
            "Discussion was not created for {}\n--> An existing discussion was found at {}",
            &post.url,
            is_existing_discussion?.unwrap()
        );
        return Ok(());
    }

    let create_disc_resp = clients
        .gql
        .post(&clients.github_gql_url)
        .run_graphql(create_disc_op)
        .await?;

    if let Some(discussion_info) = create_disc_resp
        .data
        .and_then(|d| d.create_discussion)
        .and_then(|payload| payload.discussion)
    {
        if discussion_info.title == post.url.path() {
            println!(
                "Successfully created new discussion at {} ({})",
                String::from(discussion_info.url),
                discussion_info.title
            )
        }
    } else {
        panic!(
            "Discussion could not be generated. GraphQL errors: \n{:#?}",
            create_disc_resp.errors
        );
    }
    Ok(())
}
