cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --out-dir ./dist/target --target web ./target/wasm32-unknown-unknown/release/gravity-war.wasm
cp -r assets dist
