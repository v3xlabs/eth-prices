use alloy::{primitives::BlockNumber, providers::Provider};

use crate::{
    EthPricesError,
    network::{NetworkId, NetworkInstant},
    provider::RpcProvider,
};

/// A single point-in-time on a specific network.
///
/// Each variant carries the information needed to query a rate at that moment:
///
/// | Variant | Use case | Payload |
/// |---|---|---|
/// | `EVM(chain_id, block, provider)` | On-chain asset quotes | chain ID, block height, RPC provider |
/// | `Fiat(timestamp)` | ECB / off-chain fiat rates | Unix timestamp (seconds) |
///
/// # Converting to a [`NetworkInstant`]
///
/// Use [`instant()`](NetworkTime::instant) to wrap a single [`NetworkTime`] into a
/// [`NetworkInstant`] — the type expected by [`Quoter::rate`](crate::quoter::Quoter::rate)
/// and [`Route::quote`](crate::router::Route::quote):
///
/// ```rust,ignore
/// use eth_prices::network::NetworkTime;
///
/// let time = NetworkTime::EVM(1, 20_000_000, provider);
/// let networks = time.instant();
/// ```
///
/// For multi-network scenarios (e.g. different EVM chains + a fiat timestamp) build a
/// [`NetworkInstant`] directly with its builder methods instead.
#[derive(Debug, Clone)]
pub enum NetworkTime {
    /// An EVM chain at a specific block height.
    ///
    /// Fields: `(chain_id, block_number, provider)`
    EVM(NetworkId, BlockNumber, RpcProvider),
    /// An off-chain fiat timestamp (Unix seconds).
    Fiat(u64),
}

impl NetworkTime {
    /// Extract the EVM fields, if this is the `EVM` variant.
    pub fn as_evm(&self) -> Option<(&NetworkId, &BlockNumber, &RpcProvider)> {
        match self {
            NetworkTime::EVM(chain_id, block_number, provider) => {
                Some((chain_id, block_number, provider))
            }
            _ => None,
        }
    }

    /// Extract the fiat timestamp, if this is the `Fiat` variant.
    pub fn as_fiat(&self) -> Option<&u64> {
        match self {
            NetworkTime::Fiat(date_time) => Some(date_time),
            _ => None,
        }
    }

    /// Convert this single [`NetworkTime`] into a [`NetworkInstant`].
    ///
    /// This is the simplest way to create a [`NetworkInstant`] when only one network
    /// is needed. For multi-network scenarios build the [`NetworkInstant`] directly
    /// with its default + builder methods.
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

    pub async fn from_provider_latest(
        provider: RpcProvider,
        network_id: NetworkId,
    ) -> Result<Self, EthPricesError> {
        provider
            .get_block_number()
            .await
            .map(|block_number| NetworkTime::EVM(network_id, block_number, provider))
            .map_err(EthPricesError::from)
    }

    pub fn from_provider(
        provider: RpcProvider,
        network_id: NetworkId,
        block_number: BlockNumber,
    ) -> Self {
        NetworkTime::EVM(network_id, block_number, provider)
    }
}
