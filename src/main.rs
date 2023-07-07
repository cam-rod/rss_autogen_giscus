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

use std::error::Error;

use rss_autogen_giscus::{create_discussion, discussion_exists, HttpClients, Post};

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
    let clients = HttpClients::init();
    let latest_post = Post::get_latest(&clients).await?;

    if discussion_exists(&clients, &latest_post).await {
        panic!("Discussion was not created for {}.", &latest_post.url)
    } else {
        create_discussion(&clients, latest_post).await
    }
}
