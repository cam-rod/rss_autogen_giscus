use std::ops::Deref;

use feed_rs::parser::parse;
use reqwest::Client;
use scraper::{Html, Selector};
use url::Url;

use super::HttpClients;

pub async fn latest_post(clients: &HttpClients) -> reqwest::Result<Url> {
    let rss_response = clients
        .html
        .get(&clients.website_rss_url) // https://www.wildfly.org/feed.xml
        .send()
        .await?
        .bytes()
        .await?;
    let parsed_feed =
        parse(rss_response.deref()).expect("Unable to parse team-role-org-testing feed");
    let post = parsed_feed.entries.first().expect("No posts found in feed");

    Ok(Url::parse(
        post.links
            .first()
            .expect("No link provided with first post")
            .href
            .as_str(),
    )
    .unwrap())
}

pub async fn post_description(html_client: &Client, post_url: &str) -> reqwest::Result<String> {
    let desc_selector = Selector::parse("meta[name=\"description\"]").unwrap();
    let post = Html::parse_document(&html_client.get(post_url).send().await?.text().await?);

    let desc_element = post
        .select(&desc_selector)
        .next()
        .expect("Could not find 'meta' element with name 'description'");

    Ok(desc_element
        .value()
        .attr("content")
        .expect("Invalid formatting for 'name' meta tag")
        .to_string())
}
