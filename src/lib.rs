use azure_core::HttpClient;
use azure_cosmos::prelude::*;
use azure_event_grid::{Event, EventGridClient};
use azure_storage::{blob::prelude::*, core::prelude::*};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, error::Error, result, sync::Arc};

pub type Result<T> = result::Result<T, Box<dyn Error + Send + Sync>>;

pub const EVENTGRID_VALIDATION_EVENT: &str = "Microsoft.EventGrid.SubscriptionValidationEvent";
pub const CUSTOM_EVENT_TYPE_BLOB_CREATED: &str = "DeisLabs.EventGrid.Types.BlobCreated";
pub const CUSTOM_EVENT_SUBJECT_BLOB_CREATED: &str = "DeisLabs.EventGrid.Messages.BlobCreated";

pub async fn write_blob(
    container: String,
    blob: String,
    sa: String,
    key: String,
    bytes: Vec<u8>,
    http_client: Arc<Box<dyn HttpClient>>,
) -> Result<()> {
    let blob_client = StorageAccountClient::new_access_key(http_client.clone(), sa, key)
        .as_storage_client()
        .as_container_client(container)
        .as_blob_client(blob);

    println!("Writing {} bytes.", bytes.len());

    blob_client
        .put_block_blob(bytes)
        .content_type("text/plain")
        .execute()
        .await?;

    Ok(())
}

pub async fn read_blob(
    container: String,
    blob: String,
    sa: String,
    key: String,
    http_client: Arc<Box<dyn HttpClient>>,
) -> Result<Bytes> {
    let blob_client = StorageAccountClient::new_access_key(http_client.clone(), sa, key)
        .as_storage_client()
        .as_container_client(container)
        .as_blob_client(blob);

    Ok(Bytes::from(
        blob_client.get().execute().await?.data.to_vec(),
    ))
}

pub async fn send_message<T: Serialize>(
    host: String,
    key: String,
    events: Vec<Event<T>>,
    http_client: Arc<Box<dyn HttpClient>>,
) -> Result<()> {
    let client = EventGridClient::new(host.clone(), key, http_client);
    client.publish_events(&events).await?;
    println!("Sent message to host {}", host);
    Ok(())
}

pub fn get_event_type(event: Value) -> Result<EventType> {
    let event = event.clone();
    let event_type = event.get("eventType").unwrap();

    match event_type {
        Value::String(ev) => match ev.as_str() {
            EVENTGRID_VALIDATION_EVENT => {
                let code = event
                    .get("data")
                    .expect("EventGrid validation event must contain event data")
                    .get("validationCode")
                    .expect("EventGrid validation data must contain validation code")
                    .as_str()
                    .unwrap();
                return Ok(EventType::Validation(String::from(code)));
            }
            CUSTOM_EVENT_TYPE_BLOB_CREATED => return Ok(EventType::BlobCreated),
            _ => return Ok(EventType::Custom(ev.clone())),
        },
        _ => panic!(),
    };
}

pub async fn create_collection<'a, T: 'a>(
    account: String,
    key: String,
    database: String,
    collection: String,
    doc: &'a T,
    http_client: Arc<Box<dyn HttpClient>>,
) -> Result<()>
where
    T: Serialize + CosmosEntity<'a>,
{
    let token = AuthorizationToken::primary_from_base64(&key)?;
    let client = CosmosClient::new(http_client, account, token)
        .into_database_client(database)
        .into_collection_client(collection);

    client
        .create_document()
        .is_upsert(true)
        .execute(doc)
        .await?;

    Ok(())
}

pub fn query_to_hash_map() -> Result<HashMap<String, String>> {
    let mut res = HashMap::new();

    let query = std::env::var("QUERY_STRING")?;
    for pair in query.split("&").collect::<Vec<&str>>() {
        if pair.len() == 0 {
            return Ok(res);
        }
        let q: Vec<&str> = pair.split("=").collect();
        res.insert(q[0].to_string(), q[1].to_string());
    }

    Ok(res)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventData {
    pub container: String,
    pub blob: String,
}

pub enum EventType {
    Validation(String),
    BlobCreated,
    Custom(String),
}
