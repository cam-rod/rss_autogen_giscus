use feed_rs::parser::parse;
use scraper::{Html, Selector};
use std::sync::Arc;
use url::Url;

use crate::HttpClients;

/// A representation of a typical blog post, used in creating the GitHub Discussion
pub struct Post {
    /// Title of the blog post, pulled from the `<title>` tag.
    pub title: Option<String>,

    /// Description of the blog post, pulled from the `<meta name="description">` tag.
    pub description: Option<String>,

    /// Link to the blog post.
    pub url: Url,
}

impl Post {
    /// Extracts the title and description from the latest blog post.
    pub async fn get_latest(clients: &HttpClients) -> reqwest::Result<Arc<Self>> {
        let post_url = latest_post_from_rss(clients).await?;

        let desc_selector = Selector::parse("meta[name=\"description\"]").unwrap();
        let title_selector = Selector::parse("title").unwrap();
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
        let title_element = post.select(&title_selector).next();

        Ok(Arc::new(Self {
            title: title_element.map(|title| title.text().collect::<Vec<_>>().join("")),
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
