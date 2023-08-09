use std::sync::Arc;

use feed_rs::parser::parse;
use scraper::{Html, Selector};
use url::Url;

use crate::HttpClients;

/// A representation of a typical blog post, used in creating the GitHub Discussion
#[derive(Debug)]
pub struct Post {
    /// Description of the blog post, pulled from the `<meta name="description">` tag.
    pub description: Option<String>,

    /// Link to the blog post.
    pub url: Url,
}

impl Post {
    /// Extracts the description from the latest blog post.
    pub async fn get_latest(clients: &HttpClients) -> reqwest::Result<Arc<Self>> {
        let post_url = latest_post_from_rss(clients).await?;

        let desc_selector = Selector::parse("meta[name=\"description\"]").unwrap();
        let post = Html::parse_document(
            &clients
                .html
                .get(post_url.clone())
                .send()
                .await?
                .text()
                .await?,
        );

        let desc_element = post.select(&desc_selector).next();

        Ok(Arc::new(Self {
            description: desc_element
                .and_then(|el| el.value().attr("content"))
                .map(|desc| desc.to_string()),
            url: post_url,
        }))
    }
}

/// Retrieves the latest blog post from [the website's RSS feed](HttpClients::website_rss_url).
async fn latest_post_from_rss(clients: &HttpClients) -> reqwest::Result<Url> {
    let rss_response = clients
        .html
        .get(&clients.website_rss_url)
        .send()
        .await?
        .bytes()
        .await?;
    let feed = parse(&*rss_response).expect("Unable to parse feed");

    match feed
        .entries
        .first()
        .and_then(|post| post.links.first())
        .map(|link| link.href.as_str())
    {
        Some(latest_url) => Ok(latest_url.parse().unwrap()),
        None => panic!("Unable to retrieve link to latest post from feed"),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio_test::assert_ok;

    use crate::post::latest_post_from_rss;
    use crate::{HttpClients, Post};

    const CPLX_RSS_FEED: &str = "https://rss.cbc.ca/lineup/topstories.xml";

    #[tokio::test]
    async fn test_get_post_url() {
        let clients = HttpClients::test_setup(true);
        let post = latest_post_from_rss(&clients).await;

        assert_ok!(&post);
        println!("{}", post.unwrap());
    }

    /// Try to pull the latest post from a more active RSS feed
    #[tokio::test]
    async fn test_get_post_url_complex() {
        let clients = HttpClients {
            website_rss_url: CPLX_RSS_FEED.to_string(),
            ..HttpClients::test_setup(true)
        };
        let post = latest_post_from_rss(&clients).await;

        assert_ok!(&post);
        println!("{}", post.unwrap());
    }

    #[tokio::test]
    #[should_panic]
    async fn test_invalid_rss_url() {
        let clients = HttpClients {
            website_rss_url: "https://team-role-org-testing.github.io".to_string(),
            ..HttpClients::test_setup(true)
        };

        latest_post_from_rss(&clients).await.unwrap();
    }

    #[tokio::test]
    async fn test_extract_post_details() {
        let clients = HttpClients::test_setup(true);
        post_details_internal(clients, "team-role-org-testing.github.io").await;
    }

    #[tokio::test]
    async fn test_extract_post_details_complex() {
        let clients = HttpClients {
            website_rss_url: CPLX_RSS_FEED.to_string(),
            ..HttpClients::test_setup(true)
        };
        post_details_internal(clients, "www.cbc.ca").await;
    }

    async fn post_details_internal(clients: HttpClients, post_domain: &str) {
        let post = Post::get_latest(&clients).await;

        assert_ok!(&post);
        let post = post.unwrap();
        assert_eq!(Arc::clone(&post).url.domain(), Some(post_domain));

        if post.description.as_ref().is_some() {
            if post.description.as_ref().unwrap().is_empty() {
                panic!("Description was `Some`, but length 0")
            }
        } else {
            println!("WARNING: description was not found in this post.")
        }

        println!("{:#?}", post);
    }
}
