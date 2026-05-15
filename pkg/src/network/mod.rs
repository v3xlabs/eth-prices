use alloy::{primitives::BlockNumber, providers::DynProvider};

#[derive(Debug, Clone)]
pub enum Network {
    // chain id, block number, provider
    EVM(u64, BlockNumber, DynProvider),
    // date time
    Fiat(u64),
}

impl Network {
    pub fn as_evm(&self) -> Option<(&u64, &BlockNumber, &DynProvider)> {
        match self {
            Network::EVM(chain_id, block_number, provider) => {
                Some((chain_id, block_number, provider))
            }
            _ => None,
        }
    }

    pub fn as_fiat(&self) -> Option<&u64> {
        match self {
            Network::Fiat(date_time) => Some(date_time),
            _ => None,
        }
    }
}
