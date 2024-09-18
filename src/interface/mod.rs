use async_std::future::{timeout, TimeoutError};
use ethers::{
    abi::{Abi, AbiError},
    contract::{Contract, ContractError},
    providers::{Http, Middleware, Provider},
    types::{Address, U256},
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use thiserror::Error;

#[derive(Clone)]
pub struct ChainlinkContract<'a> {
    pub contract: Contract<&'a Provider<Http>>,
    pub identifier: &'a str,
    pub decimals: u8,
    pub call_timeout: Duration,
}

#[derive(Error, Debug)]
pub enum ContractCallError<T: Middleware> {
    #[error("Abi error: {0}")]
    Abi(#[from] AbiError),
    #[error("Timeout error: {0}")]
    Timeout(#[from] TimeoutError),
    #[error("Contract error: {0}")]
    Contract(#[from] ContractError<T>),
}

/// The latest price received for this symbol.
/// This data is directly retrieved from the underlying contract.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Round {
    // Identifier of the underlying asset
    pub identifier: String,
    /// Id of the submission by the aggregator
    pub round_id: u128,
    /// Answered in round
    pub answered_in_round: u128,
    /// Timestamp for when the aggregator started collecting data
    pub started_at: U256,
    /// Timestamp for when the aggregator posted the price update
    pub updated_at: U256,
    /// Answer of this round         
    pub answer: f64,
}

/// Type alias for the raw round call to the contract
pub type RoundCall<'a> = Result<(u128, u128, U256, U256, u128), ContractError<&'a Provider<Http>>>;

#[allow(clippy::redundant_allocation)]
async fn decimals<'a>(
    contract: &ethers::contract::ContractInstance<Arc<&'a Provider<Http>>, &'a Provider<Http>>,
) -> Result<u8, ContractError<&'a Provider<Http>>> {
    Ok(contract
        .method::<_, U256>("decimals", ())
        .unwrap()
        .call()
        .await?
        .as_u64() as u8)
}

impl<'a> ChainlinkContract<'a> {
    /// Creates a new instance of a chainlink price aggregator. This is just a wrapper
    /// function to simplify the interactions with the contract.
    pub async fn new(
        provider: &'a Provider<Http>,
        identifier: &'a str,
        contract_address: Address,
        call_timeout: Duration,
    ) -> Result<ChainlinkContract<'a>, ContractCallError<&'a Provider<Http>>> {
        let abi: Abi = serde_json::from_str(include_str!("IAggregatorV3Interface.json")).unwrap();
        let contract: ethers::contract::ContractInstance<Arc<&Provider<Http>>, &Provider<Http>> =
            Contract::new(contract_address, abi, Arc::new(provider));

        let decimals = timeout(call_timeout, decimals(&contract)).await??;

        Ok(ChainlinkContract {
            contract,
            decimals,
            identifier,
            call_timeout,
        })
    }

    /// Wrapper function to call the latestRoundData method on the contract
    async fn round_data(&self) -> RoundCall<'a> {
        let round_call: RoundCall = self.contract.method("latestRoundData", ())?.call().await;
        round_call
    }

    /// Retrieves the latest price of this underlying asset
    /// from the chainlink decentralized data feed
    pub async fn latest_round_data(&self) -> Result<Round, ContractCallError<&'a Provider<Http>>> {
        // Call the contract, but timeout after 10 seconds
        let (round_id, answer, started_at, updated_at, answered_in_round) =
            timeout(self.call_timeout, self.round_data()).await??;

        // Convert the answer on contract to a string.
        let float_answer: f64 = answer.to_string().parse().unwrap();

        // Convert the contract answer into a human-readable answer
        let human_answer = float_answer / (10f64.powi(self.decimals.into()));

        Ok(Round {
            identifier: self.identifier.to_string(),
            round_id,
            answered_in_round,
            started_at,
            updated_at,
            answer: human_answer,
        })
    }
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use crate::interface::ChainlinkContract;
    use ethers::{abi::Address, providers::Provider};

    #[tokio::test]
    async fn valid_answer() {
        let provider = Provider::try_from("https://bsc-dataseed1.binance.org/").unwrap();

        let chainlink_contract = ChainlinkContract::new(
            &provider,
            "ETH",
            "0x9ef1B8c0E4F7dc8bF5719Ea496883DC6401d5b2e"
                .parse::<Address>()
                .unwrap(),
            Duration::from_secs(10),
        )
        .await
        .unwrap();
        let price_data = chainlink_contract.latest_round_data().await.unwrap();
        println!("Received data: {:#?}", price_data);
        assert!(price_data.answer.ge(&0f64));
    }
}
