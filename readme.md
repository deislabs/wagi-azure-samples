### WAGI samples using Azure services

This is a repository containing WebAssembly modules running on top of
[WAGI][wagi] (WebAssembly Gateway Interface, which allows you to run WebAssembly
WASI binaries as HTTP handlers) and using Azure services.

### What do the examples do?

There are currently two examples in this repository, `handler` and `webhook`:

- `handler` - this module reads the request body and creates a new blob in
  [Azure Blob Storage][bs], based on the container and blob name from the
  request's query string. Then, it creates a new message in [Azure
  EventGrid][ev] with the blob information (container and blob name).

- `webhook` - this module acts as a webhook [event subscription][sub] for the
  Azure EventGrid topic. It receives the event that the `handler` created, reads
  the event data containing the blob and container name, reads the blob from the
  storage account, then creates a new [Azure Cosmos DB][cosmos] collection using
  the blob bytes.

### Building and running

The following tools are required to build and run the samples:

- [`wagi`][wagi], built from [this branch][wagi-branch]
- [`just`][just]
- Rust and Cargo, with the `wasm32-wasi` target configured
- `wasm-opt` from [Binaryen][binaryen]
- [`wasmtime`][wasmtime] 0.26+ (for compiling the modules to the current
  platform - this step is optional)

```
$ rustup target add wasm32-wasi
$ just build
cargo build --release --target wasm32-wasi --bin handler
    Finished release [optimized] target(s) in 0.09s
cargo build --release --target wasm32-wasi --bin webhook
    Finished release [optimized] target(s) in 0.09s
wasm-opt target/wasm32-wasi/release/handler.wasm -O4 -o target/wasm32-wasi/release/handler.wasm
wasm-opt target/wasm32-wasi/release/webhook.wasm -O4 -o target/wasm32-wasi/release/webhook.wasm
wasmtime compile -O target/wasm32-wasi/release/handler.wasm -o target/wasm32-wasi/release/handler.wasmc
wasmtime compile -O target/wasm32-wasi/release/webhook.wasm -o target/wasm32-wasi/release/webhook.wasmc
```

Before running the sample, the modules, the following have to be configured
(create a new `wagi.toml` file with the values below):

- the Storage, EventGrid, and Cosmos DB keys and accounts
- the proper allowed hosts where the modules are allowed to make outbound
  requests to - these have to be the URL of Azure services (if no allowed hosts
  are defined for a given module, it is not allowed to make any outbound
  request).
- the `webhook` module has to be publicly available, so that EventGrid can send
  requests. Then, its endpoint (`https://<endpoint-for-wagi>/eventgrid-webhook`)
  has to be configured as a webhook subscription for the EventGrid topic where
  the `handler` sends messages (a tutorial on how to configure them through the
  portal can be found [here][sub-portal]).

```toml
[[module]]
# host = "public host"
route = "/handler"
module = "target/wasm32-wasi/release/handler.wasm"
environment = { STORAGE_ACCOUNT = "<sa>", STORAGE_MASTER_KEY = "<sa-key>", TOPIC_HOST_NAME = "<host>", TOPIC_KEY = "<host-key>" }
allowed_hosts = ["https://<storage>.blob.core.windows.net", "https://<storage>.<location>.eventgrid.azure.net"]


[[module]]
# host = "public host"
route = "/eventgrid-webhook"
module = "target/wasm32-wasi/release/webhook.wasm"
environment = { STORAGE_ACCOUNT = "<sa>", STORAGE_MASTER_KEY = "<sa-key>", COSMOS_MASTER_KEY= "<key>", COSMOS_ACCOUNT= "<account>", COSMOS_DATABASE = "<database>", COSMOS_COLLECTION = "<collection>" }
allowed_hosts = ["https://<storage>.blob.core.windows.net", "https://<cosmos>.documents.azure.com"]
```

> Note that the public endpoint can be temporarily configured through a service
> like [Ngrok][ngrok].

> If Wasmtime was used to compile the modules for the current platform, the
> extension for the modules in the WAGI config has to be changed to `.wasmc`
> (this is to distinguish the modules compiled for the current platform, and is
> totally optional and non-standard).

Start the server:

```
$ just run
wagi --config wagi.toml
```

Then, from another terminal instance:

```
$ curl '<endpoint>/handler?container=<container-name>&blob=<blob-name>' -X POST -d 'Some string data'

Writing 16 bytes.
Sent message to host https://<topic>.westus2-1.eventgrid.azure.net
```

This will execute `handler` module, which creates the blob in storage and sends
the message on the configured topic, which generates a new webhook handled by
the `webhook` module, which reads the event data, reads the blob bytes, then
creates a new collection in the database.

If everything worked properly, a new collection should have been created in
Azure Cosmos DB containing the request data:

```json
{
  "id": "1619945329548",
  "value": "Some string data"
}
```

### How does this work?

WAGI uses Wasmtime to instantiate modules that are targeting the WebAssembly
System Interface (i.e. `wasm32-wasi`). Additionally, WAGI also uses the
[`wasi-experimental-http`][wasi-experimental-http] crate to enable outbound HTTP
requests. Finally, the samples are using a branch of the [Azure Rust
SDK][rust-sdk] that is compilable to WASI.

[wagi]: https://github.com/deislabs/wagi
[wagi-branch]:
  https://github.com/radu-matei/wagi/tree/update-wasi-experimental-http
[wasmtime]: https://github.com/bytecodealliance/wasmtime
[just]: https://github.com/casey/just
[binaryen]: https://github.com/WebAssembly/binaryen
[bs]:
  https://docs.microsoft.com/en-us/azure/storage/blobs/storage-blobs-introduction
[ev]: https://docs.microsoft.com/en-us/azure/event-grid/overview
[sub]:
  https://docs.microsoft.com/en-us/azure/event-grid/concepts#event-subscriptions
[cosmos]: https://docs.microsoft.com/en-us/azure/cosmos-db/introduction
[sub-portal]:
  https://docs.microsoft.com/en-us/azure/event-grid/subscribe-through-portal
[ngrok]: https://ngrok.com/docs
[wasi-experimental-http]: https://github.com/deislabs/wasi-experimental-http/
[rust-sdk]: https://github.com/Azure/azure-sdk-for-rust
