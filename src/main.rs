//! Autogenerates GitHub Discussions to be used by Giscus.
//!
//! This came from a need to support Giscus without requiring users to authenticate
//! with the app. Since the discussion isn't created until someone comments, we needed a way to
//! automatically create it once a blog post was uploaded.
//!
//! This crate checks for the latest post in the blog's RSS feed, and then extracts the contents needed to
//! to create a post, formatted as follows:
//!
//! - **Title**: URL path of the post (not including base URL)
//! - **Description**: First paragraph of the post, followed by a full link
//!
//! This crate works best when run as a GitHub Action, triggered by the completion of the
//! `pages-build-deployment` action for GitHub pages. Since the RSS feed must be up-to-date at runtime,
//! you may need to introduce a delay.

use std::error::Error;

use rss_autogen_giscus::{create_discussion, HttpClients, Post};

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let clients = HttpClients::init();
    let latest_post = Post::get_latest(&clients).await?;

    create_discussion(clients, latest_post).await
}
