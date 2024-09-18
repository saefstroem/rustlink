# `rustlink` - Decentralized Cryptocurrency Price Feed
    
A lightweight rust library for periodically retrieving cryptocurrency prices from the ChainLink decentralized price feed. With `rustlink`, you can easily retrieve the latest price of any cryptocurrency supported by ChainLink. 

[![crates.io](https://img.shields.io/crates/v/rustlink.svg)](https://crates.io/crates/rustlink)
[![Documentation](https://docs.rs/rustlink/badge.svg)](https://docs.rs/rustlink)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![GitHub branch status](https://img.shields.io/github/checks-status/starkbamse/rustlink/main)


## Features
- Retrieve the latest price of any cryptocurrency supported by ChainLink.
- WASM-compatible, so you can use it in your web applications.
- Lightweight and easy to use.
- Customizable update interval for rate limiting.
- Add any custom contract list.
- Customizable RPC url.

## Why `rustlink`?

The core principle I have maintained while developing this library is robustness and simplicity. As Web3-developers, payment operators we often need to retrieve cryptocurrency prices. However, most of the libraries available use centralized exchange APIs that have shown to be unreliable and are affected by local exchange fluctuations. Using the ChainLink dencetralized data feed we can retrieve reliable data that is provided by many independent oracles. 

The library is made to be WASM-compatible, so you can use it in your web applications as well. This allows you to receive decentralized price updates in real-time in your web and desktop apps, without having to rely on centralized price feeds or APIs.

To see which cryptocurrencies are supported by ChainLink, you can visit the [ChainLink data feeds list](https://data.chain.link/feeds).

## What is WASM?
WebAssembly (WASM) is a binary instruction format for a stack-based virtual machine. WASM is designed as a portable target for compilation of high-level languages like C/C++/Rust, enabling deployment on the web for client and server applications. WASM is supported by all major browsers, including Chrome, Firefox, Safari, and Edge.

This means that you can run rust code in your web applications, and use `rustlink` to retrieve cryptocurrency prices in real-time in your web apps.

## Installation
To use `rustlink` in your project, add the following to your `Cargo.toml` file:

```toml
[dependencies]
rustlink = "0.0.3"
```

## Build for WASM

1. ### Prerequisites
    To build the library for WASM, you need to have the `wasm-pack` tool installed. You can install it by running the following command:

    ```bash
    cargo install wasm-pack
    ```

    You also need the wasm32-unknown-unknown target installed. You can install it by running the following command:

    ```bash
    rustup target add wasm32-unknown-unknown
    ```

2. ### Building WASM
    You can build the library either for the **browser** or for **Node.js**. 

    **For the browser:**

    ```bash
    wasm-pack build --target web --out-name rustlink --out-dir <directory>
    ```

    **For Node.js:**

    ```bash
    wasm-pack build --target nodejs --out-name rustlink --out-dir <directory>
    ```


## Usage
Here is a simple example of how you can use `rustlink` to retrieve the latest price of a cryptocurrency:

```rust
use async_std::channel::unbounded;
use rustlink::config::{Reflector, Rustlink};

#[tokio::main]
async fn main(){
    let mut contracts: Vec<(String, String)> = Vec::new();
    contracts.push((
        "ETH".to_string(),"0x9ef1B8c0E4F7dc8bF5719Ea496883DC6401d5b2e".to_string(),
    ));
    
    let (sender, receiver) = unbounded();
    
    let rustlink = Rustlink::try_new(
        "https://bsc-dataseed1.binance.org/",
        1,
        Reflector::Sender(sender),
        contracts,
    )
    .unwrap();
    rustlink.start();
    let round_data = receiver.recv().await.unwrap();
    println!("Received data: {:#?}", round_data);
}
```

You can also loop through the `receiver` to get the latest price updates in real-time by putting the receiver in a loop:

```rust
loop {
    let round_data = receiver.recv().await.unwrap();
    println!("Received data: {:#?}", round_data);
}
```

## WASM Usage

```javascript
import init, { RustlinkJS } from '../web/rustlink.js';

async function runWasm() {
   await init(); // Initialize the wasm module

   // Example data
   const rpcUrl = "https://bsc-dataseed1.binance.org/";
   const fetchIntervalSeconds = BigInt(1);
   const contracts = [
       ["ETH", "0x9ef1B8c0E4F7dc8bF5719Ea496883DC6401d5b2e"],
       ["1INCH", "0x9a177Bb9f5b6083E962f9e62bD21d4b5660Aeb03"],
   ];

   async function callback(roundData) {
       console.log("Callback received:", roundData);
   }

   let rustlink = new RustlinkJS(rpcUrl, fetchIntervalSeconds, contracts, callback);

   rustlink.start();
   console.log("Stopping after 5 seconds");
   setTimeout(() => {
       rustlink.stop();
   }, 5000);
}

runWasm();
```
