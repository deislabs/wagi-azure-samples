use azure_core::{HttpClient, WasiHttpClient};
use azure_event_grid::Event;
use common::*;
use futures::executor::block_on;
use std::sync::Arc;

pub fn main() {
    println!("Content-Type: text/plain\n");

    block_on(run()).unwrap();
}

pub async fn run() -> Result<()> {
    let (container, blob) = container_and_blob_from_query()?;
    let (sa, sa_key, host, host_key) = keys_from_env()?;
    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(WasiHttpClient {}));

    let mut buf = Vec::new();
    std::io::copy(&mut std::io::stdin(), &mut buf)?;

    write_blob(
        container.clone(),
        blob.clone(),
        sa,
        sa_key,
        buf,
        http_client.clone(),
    )
    .await?;

    let events = vec![Event::new(
        None,
        CUSTOM_EVENT_TYPE_BLOB_CREATED,
        CUSTOM_EVENT_SUBJECT_BLOB_CREATED,
        EventData { container, blob },
        None,
    )];

    send_message(host, host_key, events, http_client).await?;

    Ok(())
}

fn container_and_blob_from_query() -> Result<(String, String)> {
    let qs = query_to_hash_map()?;
    let container = qs.get("container").unwrap().clone();
    let blob = qs.get("blob").unwrap().clone();

    Ok((container, blob))
}

fn keys_from_env() -> Result<(String, String, String, String)> {
    Ok((
        std::env::var("STORAGE_ACCOUNT")?,
        std::env::var("STORAGE_MASTER_KEY")?,
        std::env::var("TOPIC_HOST_NAME")?,
        std::env::var("TOPIC_KEY")?,
    ))
}
