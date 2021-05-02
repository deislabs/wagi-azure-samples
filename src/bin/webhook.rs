use azure_core::{HttpClient, WasiHttpClient};
use azure_cosmos::prelude::*;
use common::*;
use futures::executor::block_on;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

fn main() {
    println!("Content-Type: text/plain\n");

    block_on(run()).unwrap();
}

async fn run() -> Result<()> {
    // Event Grid sends the events to subscribers in an array that has a single event.
    // https://docs.microsoft.com/en-us/azure/event-grid/event-schema
    let event: Vec<Value> = serde_json::from_reader(&mut std::io::stdin())?;
    let event = event[0].clone();

    match get_event_type(event.clone())? {
        EventType::Validation(code) => {
            let val = json!({
                "validationResponse": code,
            });
            println!("{}", val);
            return Ok(());
        }
        EventType::BlobCreated => return handle_blob_created_event(event).await,
        EventType::Custom(ev) => {
            panic!("unknown event {}", ev)
        }
    }
}

async fn handle_blob_created_event(event: Value) -> Result<()> {
    let data: EventData = serde_json::from_value(
        event
            .get("data")
            .expect("EventGrid events must contain a data section")
            .clone(),
    )?;

    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(WasiHttpClient {}));

    let (sa, key, cosmos_account, cosmos_key, database, collection) = keys_from_env()?;
    let bytes = read_blob(
        data.container.clone(),
        data.blob.clone(),
        sa,
        key,
        http_client.clone(),
    )
    .await?;

    let doc = Entity {
        id: chrono::Utc::now().timestamp_millis().to_string(),
        value: String::from_utf8(bytes.to_vec())?,
    };

    create_collection(
        cosmos_account,
        cosmos_key,
        database,
        collection,
        &doc,
        http_client,
    )
    .await?;

    Ok(())
}

fn keys_from_env() -> Result<(String, String, String, String, String, String)> {
    Ok((
        std::env::var("STORAGE_ACCOUNT")?,
        std::env::var("STORAGE_MASTER_KEY")?,
        std::env::var("COSMOS_ACCOUNT")?,
        std::env::var("COSMOS_MASTER_KEY")?,
        std::env::var("COSMOS_DATABASE")?,
        std::env::var("COSMOS_COLLECTION")?,
    ))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Entity {
    id: String,
    value: String,
}

impl<'a> CosmosEntity<'a> for Entity {
    type Entity = &'a str;

    fn partition_key(&'a self) -> Self::Entity {
        self.id.as_ref()
    }
}
