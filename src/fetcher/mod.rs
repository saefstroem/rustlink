use std::time::Duration;

use async_std::stream::StreamExt;
use ethers::providers::{Http, Provider};
use ethers::types::Address;
use futures::{select, FutureExt};

use super::interface::{ChainlinkContract, Round};
use crate::core::Reflector::Sender;
use crate::core::{Configuration, Rustlink};
use crate::interface::ContractCallError;

/// Retrieves the price of an underlying asset from a particular contract
async fn fetch_round_data_for_contract<'a>(
    rustlink_configuration: &'a Configuration,
    identifier: &'a str,
    address: Address,
) -> Result<Round, ContractCallError<&'a Provider<Http>>> {
    let contract = ChainlinkContract::new(
        &rustlink_configuration.provider,
        identifier,
        address,
        rustlink_configuration.call_timeout,
    )
    .await?;
    contract.latest_round_data().await
}

// The function signature looks good, but ensure all types (Rustlink, Round, etc.) are properly defined.
pub async fn fetch_rounds(rustlink: Rustlink) {
    let contracts = &rustlink.configuration.contracts;
    let mut shutdown_future = rustlink.termination_recv.recv().fuse();
    let worker_future = workflow_rs::core::task::interval(Duration::from_secs(
        rustlink.configuration.fetch_interval_seconds,
    ));
    futures::pin_mut!(worker_future);

    // This loop runs indefinitely, fetching price data.
    loop {
        for contract_configuration in contracts {
            select! {
                    _ = shutdown_future => {
                        rustlink.shutdown_send.send(()).await.unwrap();
                        return;
                    },
                    _ = worker_future.next().fuse() => {

                let identifier = &contract_configuration.0; // This variable wasn't used in your original code.
                let address = &contract_configuration.1;

                // Fetch price data and attempt to send it via the channel.
                match fetch_round_data_for_contract(&rustlink.configuration, identifier, *address).await
                {
                    Ok(price_data) => {
                        match rustlink.reflector {
                            Sender(ref sender) => {
                                // Attempt to send the PriceData through the channel.
                                if let Err(error) = sender.send(price_data).await {
                                    log::error!("Failed sending data: {}", error);
                                }
                            }
                        }
                    }
                    Err(error) => {
                        log::error!("Failed updating price: {}", error);
                    }
                }
            }
            }
        }
    }
}
