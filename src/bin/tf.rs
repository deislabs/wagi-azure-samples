use azure_core::{HttpClient, WasiHttpClient};
use azure_cosmos::responses::QueryResult;
use common::*;
use futures::executor::block_on;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::BufRead;
use tract_tensorflow::prelude::*;

const MOBILENET_V2: &str = "mobilenet_v2_1.4_224_frozen.pb";
const LABELS: &str = "labels.txt";

fn main() {
    println!("Content-Type: text/plain\n");

    block_on(run()).unwrap();
}

async fn run() -> Result<()> {
    let mut bytes = Vec::new();
    std::io::copy(&mut std::io::stdin(), &mut bytes)?;

    let mut hasher = Sha256::new();
    hasher.update(bytes.clone());
    let res = hasher.finalize();
    let digest = format!("{:X}", res);

    let http_client: Arc<Box<dyn HttpClient>> = Arc::new(Box::new(WasiHttpClient {}));
    let (account, key, database, collection) = keys_from_env()?;

    let res = match check_cache(
        digest.clone(),
        account.clone(),
        key.clone(),
        database.clone(),
        collection.clone(),
        http_client.clone(),
    )
    .await?
    {
        Some(res) => res,
        None => {
            predict_and_write(
                bytes,
                digest,
                account,
                key,
                database,
                collection,
                http_client,
            )
            .await?
        }
    };

    println!("{}", res);
    Ok(())
}

async fn check_cache(
    digest: String,
    account: String,
    key: String,
    database: String,
    collection: String,
    http_client: Arc<Box<dyn HttpClient>>,
) -> Result<Option<String>> {
    let query = format!("SELECT * FROM c where c.id = '{}' ", digest);

    let res =
        query_collection::<Entity>(account, key, database, collection, query, http_client).await?;

    match res.results.get(0) {
        Some(res) => match res {
            QueryResult::Document(doc) => {
                let res = &doc.result.value;
                Ok(Some(res.to_string()))
            }
            QueryResult::Raw(raw) => {
                let res = &raw.value;
                Ok(Some(res.to_string()))
            }
        },
        None => Ok(None),
    }
}

fn runnable_model(
) -> Result<SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>> {
    let model = tract_tensorflow::tensorflow()
        // load the model
        .model_for_path(MOBILENET_V2)?
        // specify input type and shape
        .with_input_fact(
            0,
            InferenceFact::dt_shape(f32::datum_type(), tvec!(1, 224, 224, 3)),
        )?
        // optimize the model
        .into_optimized()?
        // make the model runnable and fix its inputs and outputs
        .into_runnable()?;
    Ok(model)
}

fn run_model(
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
    bytes: Vec<u8>,
) -> Result<Option<(f32, i32)>> {
    let image = image::load_from_memory(&bytes)?;
    let image = image::imageops::resize(&image, 224, 224, ::image::imageops::FilterType::Triangle);
    let image: Tensor = tract_ndarray::Array4::from_shape_fn((1, 224, 224, 3), |(_, y, x, c)| {
        image[(x as _, y as _)][c] as f32 / 255.0
    })
    .into();

    let result = model.run(tvec!(image))?;
    let best = result[0]
        .to_array_view::<f32>()?
        .iter()
        .cloned()
        .zip(1..)
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    Ok(best)
}

async fn predict_and_write(
    bytes: Vec<u8>,
    digest: String,
    account: String,
    key: String,
    database: String,
    collection: String,
    http_client: Arc<Box<dyn HttpClient>>,
) -> Result<String> {
    let model = runnable_model()?;
    let (cert, label) = run_model(model, bytes)
        .expect("cannot get prediction")
        .unwrap();

    let label = get_label(label as usize)?;

    let res = format!(
        "The image represents a {}, with {}% accuracy",
        label,
        cert * 100.0
    );

    let doc = Entity {
        id: digest,
        value: res.clone(),
    };

    create_collection(account, key, database, collection, &doc, http_client).await?;

    Ok(res)
}

fn get_label(num: usize) -> Result<String> {
    // The result of executing the inference is the predicted class,
    // which also indicates the line number in the (1-indexed) labels file.
    let labels = File::open(LABELS.to_string())?;
    let content = std::io::BufReader::new(&labels);
    let label = content
        .lines()
        .nth(num - 1)
        .expect("cannot find line in labels file");
    Ok(label?)
}

fn keys_from_env() -> Result<(String, String, String, String)> {
    Ok((
        std::env::var("COSMOS_ACCOUNT")?,
        std::env::var("COSMOS_MASTER_KEY")?,
        std::env::var("COSMOS_DATABASE")?,
        std::env::var("COSMOS_COLLECTION")?,
    ))
}
