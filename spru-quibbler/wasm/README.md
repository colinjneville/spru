# Building and running locally
* cargo build --verbose --release -p spru-quibbler --target wasm32-unknown-unknown --no-default-features --features "hotseat,join"
* wasm-bindgen --out-name wasm-quibbler --out-dir ./spru-quibbler/wasm --target web ./target/wasm32-unknown-unknown/release/spru-quibbler.wasm
* miniserve ./spru-quibbler/wasm --index quibbler.html -p 8080 -i 127.0.0.1

# First-time setup
* cargo install miniserve
* TODO install wasm32-unknown-unknown target
* TODO install wasm-bindgen

# File size analysis
* twiggy top ./spru-quibbler/wasm/wasm-quibbler_bg.wasm >> twiggy.txt
- https://alexene.dev/twiggy/usage/index.html