/*!
Network timeee
*/

use std::collections::HashMap;

use alloy::{
    primitives::BlockNumber,
    providers::{DynProvider, Provider},
};

use crate::EthPricesError;

pub type NetworkId = u64;

/// An combination of a given network, block height / time, and provider.
#[derive(Debug, Clone)]
pub enum NetworkTime {
    // chain id, block number, provider
    EVM(u64, BlockNumber, DynProvider),
    // date time
    Fiat(u64),
}

impl NetworkTime {
    pub fn as_evm(&self) -> Option<(&NetworkId, &BlockNumber, &DynProvider)> {
        match self {
            NetworkTime::EVM(chain_id, block_number, provider) => {
                Some((chain_id, block_number, provider))
            }
            _ => None,
        }
    }

    pub fn as_fiat(&self) -> Option<&u64> {
        match self {
            NetworkTime::Fiat(date_time) => Some(date_time),
            _ => None,
        }
    }
}

/// Network Instant
/// Due to the nature of how eth-prices quotes across different networks, a `NetworkInstant` is used to keep track of the different `NetworkTime`s and ensure consistent backquerying against a custom blockheight or date.
#[derive(Default, Debug, Clone)]
pub struct NetworkInstant(pub HashMap<NetworkId, NetworkTime>);

impl NetworkInstant {

    pub fn get(&self, network_id: &NetworkId) -> Option<&NetworkTime> {
        self.0.get(network_id)
    }

    pub fn get_evm_block(
        &self,
        network_id: NetworkId,
    ) -> Option<(&NetworkId, &BlockNumber, &DynProvider)> {
        self.0
            .get(&network_id)
            .and_then(|network_time| network_time.as_evm())
    }

    pub fn get_fiat_timestamp(&self) -> Option<&u64> {
        self.0
            .get(&0)
            .and_then(|network_time| network_time.as_fiat())
    }

    pub fn with_fiat_timestamp(mut self, timestamp: u64) -> Self {
        self.0.insert(0, NetworkTime::Fiat(timestamp));
        self
    }

    pub fn with_evm_block(mut self, network_id: NetworkId, block_number: BlockNumber, provider: DynProvider) -> Self {
        self.0.insert(network_id, NetworkTime::EVM(network_id, block_number, provider));
        self
    }

    pub async fn with_evm_latest(mut self, network_id: NetworkId, provider: DynProvider) -> Result<Self, EthPricesError> {
        let block_number = provider.get_block_number().await?;
        self.0.insert(network_id, NetworkTime::EVM(network_id, block_number, provider));
        Ok(self)
    }

    pub async fn with_evm_provider(mut self, provider: DynProvider) -> Result<Self, EthPricesError> {
        let block_number = provider.get_block_number().await?;
        self.0.insert(0, NetworkTime::EVM(0, block_number, provider));
        Ok(self)
    }
}
