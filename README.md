# Gravity War

[image]

Little game for the [Bevy Jam #3](https://itch.io/jam/bevy-jam-3)

## Game

[description]

## Commands

[commands]

## Build

1. Compile wasm app

```sh
cargo build --release --target wasm32-unknown-unknown
```

2. Create JS bindings

```sh
wasm-bindgen --out-dir ./dist/target --target web ./target/wasm32-unknown-unknown/release/gravity-war.wasm
```

3. Copy assets to the `dist` folder

```sh
cp -r assets dist
```

## Publish

- Start a local web server

```sh
basic-http-server dist
```

- Publish to itch.io

```sh
zip dist.zip dist/**/*
```

## Licence

Code is licensed under MIT or Apache-2.0.  
Assets are licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
