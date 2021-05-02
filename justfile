build:
    cargo build --release --target wasm32-wasi --bin handler
    cargo build --release --target wasm32-wasi --bin webhook
    cargo build --release --target wasm32-wasi --bin tf
    wasm-opt target/wasm32-wasi/release/handler.wasm -O4 -o target/wasm32-wasi/release/handler.wasm
    wasm-opt target/wasm32-wasi/release/webhook.wasm -O4 -o target/wasm32-wasi/release/webhook.wasm
    wasm-opt target/wasm32-wasi/release/tf.wasm -O4 -o target/wasm32-wasi/release/tf.wasm
    wasmtime compile -O target/wasm32-wasi/release/handler.wasm -o target/wasm32-wasi/release/handler.wasmc
    wasmtime compile -O target/wasm32-wasi/release/webhook.wasm -o target/wasm32-wasi/release/webhook.wasmc
    wasmtime compile -O target/wasm32-wasi/release/tf.wasm -o target/wasm32-wasi/release/tf.wasmc

run:
    wagi --config wagi.toml
