use feed_rs::parser::parse;
use scraper::{Html, Selector};
use url::Url;

use crate::HttpClients;

pub struct Post {
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Url,
}

impl Post {
    pub async fn get_latest(clients: &HttpClients) -> reqwest::Result<Self> {
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

        Ok(Self {
            title: title_element.map(|title| title.text().collect::<Vec<_>>().join("")),
            description: desc_element
                .and_then(|el| el.value().attr("content"))
                .map(|desc| desc.to_string()),
            url: post_url,
        })
    }
}

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
