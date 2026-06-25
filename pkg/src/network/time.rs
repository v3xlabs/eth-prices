use alloy::primitives::BlockNumber;

use crate::{
    network::{NetworkId, NetworkInstant},
    provider::RpcProvider,
};

/// An combination of a given network, block height / time, and provider.
#[derive(Debug, Clone)]
pub enum NetworkTime {
    // chain id, block number, provider
    EVM(u64, BlockNumber, RpcProvider),
    // date time
    Fiat(u64),
}

impl NetworkTime {
    pub fn as_evm(&self) -> Option<(&NetworkId, &BlockNumber, &RpcProvider)> {
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

    /// Function for converting a single network into a NetworkInstant
    /// This can be usefull if you only want to query a single network
    pub fn instant(self) -> NetworkInstant {
        match self {
            NetworkTime::EVM(network_id, block_number, provider) => {
                NetworkInstant::default().with_evm_block(network_id, block_number, provider)
            }
            NetworkTime::Fiat(date_time) => {
                NetworkInstant::default().with_fiat_timestamp(date_time)
            }
        }
    }
}
