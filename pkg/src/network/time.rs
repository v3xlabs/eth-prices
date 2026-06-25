use alloy::primitives::BlockNumber;

use crate::{network::NetworkId, provider::RpcProvider};

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
}
