use crate::{error::Error, fetcher::fetch_rounds, interface};

use async_std::channel::{unbounded, Receiver, RecvError, Sender};
use ethers::{
    providers::{Http, Provider},
    types::Address,
};
use js_sys::Function;
use serde_wasm_bindgen::{from_value, to_value};
use std::str::FromStr;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use wasm_bindgen_futures::spawn_local;
use workflow_rs::core::cfg_if;

/// ## Configuration
/// This struct contains the configuration for Rustlink. It contains the following fields:
/// - `fetch_interval_seconds`: How often to update data points (to prevent RPC rate limitation)
/// - `contracts`: A list of tuples containing a ticker name and its corresponding contract address on the EVM chain
/// - `provider`: The provider to use for fetching data
#[derive(Clone)]
pub struct Configuration {
    pub fetch_interval_seconds: u64,
    pub contracts: Vec<(String, Address)>,
    pub provider: Provider<Http>,
    pub call_timeout: std::time::Duration,
}

/// ## Rustlink instance. This is the main struct that you will interact with.
///
/// Rustlink is a lightweight Rust library that provides your Rust applications with a direct
/// link to the latest cryptocurrency prices. All data is retrieved from Chainlink decentralized
/// price feeds. Just copy the contract addresses for the symbol that you would like to track from:
///
#[derive(Clone)]
pub struct Rustlink {
    pub configuration: Configuration,
    pub reflector: Reflector,
    pub termination_send: Sender<()>,
    pub termination_recv: Receiver<()>,
    pub shutdown_send: Sender<()>,
    pub shutdown_recv: Receiver<()>,
}

/// Rustlink allows you as a developer to retrieve the sought Chainlink data in
/// different ways.
///
/// In its current state you can pass a Sender from an unbound async-std channel
/// which you can create by doing:
/// ```rust
/// use async_std::channel::unbounded;
/// use rustlink::core::Reflector;
///
/// let (sender, receiver) = unbounded();
///
/// let reflector=Reflector::Sender(sender);
/// ```
///
/// You may clone the receiver as many times as you want but do not use the sender
/// for anything other than passing it to the try_new() method.
#[derive(Clone)]
pub enum Reflector {
    /// A sender from async-std
    Sender(Sender<Round>),
}

pub type Round = interface::Round;

impl Rustlink {
    /// Creates a new Rustlink instance.
    ///
    /// Expected parameters:
    /// - `rpc_url`: The RPC url of your chosen EVM network where Chainlink offers decentralised data feeds.
    /// - `fetch_interval_seconds`: How often to update data points in the database (to prevent RPC rate limitation)
    /// - `reflector`: How you choose to receive the answer from your provided contracts.
    /// - `contracts`: A tuple list containing a ticker name and its corresponding contract address on the
    ///   EVM chain.
    ///
    /// Example:
    ///
    /// ```rust
    /// use async_std::channel::unbounded;
    /// use rustlink::core::{Reflector, Rustlink};
    ///
    /// #[tokio::main]
    ///
    /// async fn main(){
    ///     let mut contracts: Vec<(String, String)> = Vec::new();
    ///     contracts.push((
    ///         "ETH".to_string(),
    ///         "0x9ef1B8c0E4F7dc8bF5719Ea496883DC6401d5b2e".to_string(),
    ///     ));
    ///     
    ///     let (sender, receiver) = unbounded();
    ///     
    ///     let rustlink = Rustlink::try_new(
    ///         "https://bsc-dataseed1.binance.org/",
    ///         1,
    ///         Reflector::Sender(sender),
    ///         contracts,
    ///         std::time::Duration::from_secs(10),
    ///     )
    ///     .unwrap();
    ///     rustlink.start();
    ///     let round_data = receiver.recv().await.unwrap();
    ///     println!("Received data: {:#?}", round_data);
    /// }
    /// ```
    pub fn try_new(
        rpc_url: &str,
        fetch_interval_seconds: u64,
        reflector: Reflector,
        contracts: Vec<(String, String)>,
        call_timeout: std::time::Duration,
    ) -> Result<Self, Error> {
        let provider = Provider::try_from(rpc_url).expect("Invalid RPC URL");
        let (termination_send, termination_recv) = unbounded::<()>();
        let (shutdown_send, shutdown_recv) = unbounded::<()>();

        let parsed_contracts = contracts
            .iter()
            .map(|(identifier, address)| {
                (
                    identifier.clone(),
                    Address::from_str(address).expect("Invalid contract address specified"),
                )
            })
            .collect();

        Ok(Rustlink {
            configuration: Configuration {
                fetch_interval_seconds,
                provider,
                contracts: parsed_contracts,
                call_timeout,
            },
            reflector,
            termination_send,
            termination_recv,
            shutdown_send,
            shutdown_recv,
        })
    }

    /// Starts the Rustlink instance.
    /// This method will start fetching the latest price data from the Chainlink decentralized data feed.
    pub fn start(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        tokio::task::spawn(fetch_rounds(self.clone()));

        #[cfg(target_arch = "wasm32")]
        async_std::task::block_on(fetch_rounds(self.clone()));
    }

    /// Stops the Rustlink instance.
    /// This method will stop fetching the latest price data from the Chainlink decentralized data feed.
    pub async fn stop(&self) -> Result<(), RecvError> {
        self.termination_send.send(()).await.unwrap();
        self.shutdown_recv.recv().await
    }
}

/// RustlinkJS is a JavaScript wrapper for Rustlink.
/// It allows you to create a Rustlink instance in JavaScript and start fetching data when you use WASM.
/// You should use this one when you want to use Rustlink in a web environment.
#[wasm_bindgen]
pub struct RustlinkJS {
    rustlink: Rustlink,
    callback: Function,
    receiver: Receiver<Round>,
}

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        #[wasm_bindgen(typescript_custom_section)]
        const TS_CONTRACTS: &'static str = r#"
        /** 
         * A contract tuple containing an identifier and a contract address. 
         * 
         * **Order matters.**
         * Example
         * ```typescript
         * let contracts=[["Ethereum","0x9ef1B8c0E4F7dc8bF5719Ea496883DC6401d5b2e"]]
         * ```
        */
        export type Contract = [string,string] 
        "#;

    }
}

#[wasm_bindgen]
extern "C" {
    // A JavaScript array of contract tuples
    #[wasm_bindgen(extends = js_sys::Function, typescript_type = "Contract[]")]
    pub type Contracts;
}

#[wasm_bindgen]
impl RustlinkJS {
    /// Creates a new RustlinkJS instance.
    /// Expected parameters:
    /// - `rpc_url`: The RPC url of your chosen EVM network where Chainlink offers decentralised data feeds.
    /// - `fetch_interval_seconds`: How often to update data points (to prevent RPC rate limitation)
    /// - `contracts`: A list of tuples containing a ticker name and its corresponding contract address on the EVM chain
    /// - `callback`: A JavaScript function (async or sync) that will be called every time a new data point is fetched
    /// ```javascript
    /// import init, { RustlinkJS } from '../web/rustlink.js';
    ///
    /// async function runWasm() {
    ///    await init(); // Initialize the wasm module
    ///
    ///    // Example data
    ///    const rpcUrl = "https://bsc-dataseed1.binance.org/";
    ///    const fetchIntervalSeconds = BigInt(1);
    ///    const contracts = [
    ///        ["ETH", "0x9ef1B8c0E4F7dc8bF5719Ea496883DC6401d5b2e"],
    ///        ["1INCH", "0x9a177Bb9f5b6083E962f9e62bD21d4b5660Aeb03"],
    ///    ];
    ///
    ///    async function callback(roundData) {
    ///        console.log("Callback received:", roundData);
    ///    }
    ///
    ///    let rustlink = new RustlinkJS(rpcUrl, fetchIntervalSeconds, contracts, callback);
    ///
    ///    rustlink.start();
    ///    console.log("Stopping after 5 seconds");
    ///    setTimeout(() => {
    ///        rustlink.stop();
    ///    }, 5000);
    /// }
    ///
    /// runWasm();
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(
        rpc_url: &str,
        fetch_interval_seconds: u64,
        contracts: Contracts,
        callback: Function,
    ) -> Self {
        // Cast `JsValue` to `Function`

        let contracts: Vec<(String, String)> = from_value(contracts.into()).unwrap();

        let (sender, receiver) = async_std::channel::unbounded();
        let reflector = Reflector::Sender(sender);
        let rustlink = Rustlink::try_new(
            rpc_url,
            fetch_interval_seconds,
            reflector,
            contracts,
            std::time::Duration::from_secs(10),
        )
        .map_err(|e| JsValue::from_str(&format!("{}", e)))
        .unwrap();

        RustlinkJS {
            rustlink,
            callback,
            receiver,
        }
    }

    /// Starts the RustlinkJS instance.
    /// This method will start fetching the latest price data from the Chainlink decentralized data feed.
    #[wasm_bindgen]
    pub fn start(&self) {
        self.rustlink.start();
        let receiver = self.receiver.clone();
        let callback = self.callback.clone();
        spawn_local(async move {
            while let Ok(round) = receiver.recv().await {
                // Prepare arguments to pass to JS function
                let this = JsValue::NULL; // 'this' context for function, null in this case
                let arg_js = to_value(&round).unwrap();

                // Call the function
                let _ = callback.call1(&this, &arg_js);
            }
        });
    }

    /// Stops the RustlinkJS instance.
    /// This method will stop fetching the latest price data from the Chainlink decentralized data feed.
    #[wasm_bindgen]
    pub async fn stop(&self) -> Result<(), JsValue> {
        self.rustlink
            .stop()
            .await
            .map_err(|e| JsValue::from_str(&format!("Shutdown error: {}", e)))
    }
}
