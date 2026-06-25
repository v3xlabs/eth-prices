use std::collections::HashMap;

use alloy::{primitives::BlockNumber, providers::Provider};

use crate::{EthPricesError, network::{NetworkId, NetworkTime}, provider::RpcProvider};


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
    ) -> Option<(&NetworkId, &BlockNumber, &RpcProvider)> {
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

    pub fn with_evm_block(mut self, network_id: NetworkId, block_number: BlockNumber, provider: RpcProvider) -> Self {
        self.0.insert(network_id, NetworkTime::EVM(network_id, block_number, provider));
        self
    }

    pub async fn with_evm_latest(mut self, network_id: NetworkId, provider: RpcProvider) -> Result<Self, EthPricesError> {
        let block_number = provider.get_block_number().await?;
        self.0.insert(network_id, NetworkTime::EVM(network_id, block_number, provider));
        Ok(self)
    }

    pub async fn with_evm_provider(mut self, provider: RpcProvider) -> Result<Self, EthPricesError> {
        let block_number = provider.get_block_number().await?;
        self.0.insert(0, NetworkTime::EVM(0, block_number, provider));
        Ok(self)
    }
}
