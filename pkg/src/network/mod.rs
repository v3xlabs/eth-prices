use std::collections::HashMap;

use alloy::{
    primitives::BlockNumber,
    providers::{DynProvider, Provider},
};

use crate::EthPricesError;

pub type NetworkId = u64;

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

pub struct NetworkTimes(pub HashMap<NetworkId, NetworkTime>);

impl NetworkTimes {
    pub fn get(&self, network_id: NetworkId) -> Option<&NetworkTime> {
        self.0.get(&network_id)
    }

    pub fn get_evm(
        &self,
        network_id: NetworkId,
    ) -> Option<(&NetworkId, &BlockNumber, &DynProvider)> {
        self.0
            .get(&network_id)
            .and_then(|network_time| network_time.as_evm())
    }

    pub fn get_fiat(&self) -> Option<&u64> {
        self.0
            .get(&0)
            .and_then(|network_time| network_time.as_fiat())
    }

    pub fn from_provider(
        provider: DynProvider,
        network_id: NetworkId,
        block_number: BlockNumber,
    ) -> Self {
        Self(HashMap::from([(
            network_id,
            NetworkTime::EVM(network_id, block_number, provider),
        )]))
    }

    pub async fn from_provider_latest(
        provider: DynProvider,
        network_id: NetworkId,
    ) -> Result<Self, EthPricesError> {
        let block_number = provider.get_block_number().await?;
        Ok(Self(HashMap::from([(
            network_id,
            NetworkTime::EVM(network_id, block_number, provider),
        )])))
    }
}
