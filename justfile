build:
    cargo build --release --target wasm32-wasi --bin handler
    cargo build --release --target wasm32-wasi --bin webhook
    wasm-opt target/wasm32-wasi/release/handler.wasm -O4 -o target/wasm32-wasi/release/handler.wasm
    wasm-opt target/wasm32-wasi/release/webhook.wasm -O4 -o target/wasm32-wasi/release/webhook.wasm
    wasmtime compile -O target/wasm32-wasi/release/handler.wasm -o target/wasm32-wasi/release/handler.wasmc
    wasmtime compile -O target/wasm32-wasi/release/webhook.wasm -o target/wasm32-wasi/release/webhook.wasmc

run:
    wagi --config wagi.toml
