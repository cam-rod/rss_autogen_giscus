use cynic::http::CynicReqwestError;

use rss_autogen_giscus::{create_discussion, HttpClients, Post};

#[tokio::main]
pub async fn main() -> Result<(), CynicReqwestError> {
    let clients = HttpClients::init();
    let latest_post = Post::get_latest(&clients).await?;

    create_discussion(clients, latest_post).await
}
