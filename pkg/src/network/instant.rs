use std::collections::HashMap;

use alloy::{primitives::BlockNumber, providers::Provider};

use crate::{
    EthPricesError,
    network::{Network, NetworkId, NetworkTime},
    provider::RpcProvider,
};

/// A snapshot of network states across multiple chains / data sources.
///
/// Internally a `HashMap<NetworkId, NetworkTime>`, where:
///
/// - `NetworkId` is a `u64` — for EVM chains this is the chain ID; for fiat data the
///   sentinel value `0` is used.
/// - [`NetworkTime`] stores the actual point-in-time (block height or unix timestamp)
///   together with the provider needed to query it.
///
/// # Construction — Builder Pattern
///
/// Start with [`Default::default()`] and chain builder methods. Each builder consumes
/// and returns `self`, enabling a fluent style:
///
/// ```rust,ignore
/// use eth_prices::network::NetworkInstant;
///
/// let networks = NetworkInstant::default()
///     .with_evm_block(1, 20_000_000, eth_provider)
///     .with_evm_block(42161, 200_000_000, arb_provider)
///     .with_fiat_timestamp(1_700_000_000);
/// ```
///
/// For single-network convenience, start with [`NetworkTime::instant()`] instead.
///
/// # Reuse
///
/// [`NetworkInstant`] implements [`Clone`]. Build one at the start of a request and
/// share it across all [`Route::quote`](crate::router::Route::quote) calls so every
/// sub-quote uses consistent block heights and providers.
#[derive(Default, Debug, Clone)]
pub struct NetworkInstant(pub HashMap<Network, NetworkTime>);

impl NetworkInstant {
    /// Look up the [`NetworkTime`] for a given network.
    pub fn get(&self, network_id: &Network) -> Option<&NetworkTime> {
        self.0.get(network_id)
    }

    /// Convenience accessor: extract the EVM fields for a specific chain.
    ///
    /// Returns `(chain_id, block_number, provider)` if the network exists and is
    /// an `EVM` variant.
    pub fn get_evm_block(
        &self,
        network_id: Network,
    ) -> Option<(&NetworkId, &BlockNumber, &RpcProvider)> {
        self.0
            .get(&network_id)
            .and_then(|network_time| network_time.as_evm())
    }

    /// Convenience accessor: extract the fiat timestamp (network id `0`).
    pub fn get_fiat_timestamp(&self) -> Option<&u64> {
        self.0
            .get(&Network::Fiat)
            .and_then(|network_time| network_time.as_fiat())
    }

    // ── Builder methods ──────────────────────────────────────────────────────────

    /// Set a fiat timestamp at network id `0`.
    ///
    /// Used by [`ecb`](crate::quoter::ecb) quoters to pick a date.
    pub fn with_fiat_timestamp(mut self, timestamp: u64) -> Self {
        self.0.insert(Network::Fiat, NetworkTime::Fiat(timestamp));
        self
    }

    /// Set an EVM network to a specific block number.
    ///
    /// Use this when you already know the block height (e.g. from a historical query
    /// or configuration).
    pub fn with_evm_block(
        mut self,
        network_id: NetworkId,
        block_number: BlockNumber,
        provider: RpcProvider,
    ) -> Self {
        self.0.insert(
            network_id.clone().into(),
            NetworkTime::EVM(network_id, block_number, provider),
        );
        self
    }

    /// Set an EVM network to the latest block, fetched from the RPC.
    ///
    /// This is an async builder — it must be `.await`ed before further chaining:
    ///
    /// ```rust,ignore
    /// let networks = NetworkInstant::default()
    ///     .with_evm_latest(1, eth_provider).await?
    ///     .with_evm_latest(42161, arb_provider).await?;
    /// ```
    pub async fn with_evm_latest(
        mut self,
        network_id: NetworkId,
        provider: RpcProvider,
    ) -> Result<Self, EthPricesError> {
        self.0.insert(
            network_id.clone().into(),
            NetworkTime::from_provider_latest(provider, network_id).await?,
        );
        Ok(self)
    }

    pub async fn with_evm_provider(
        mut self,
        provider: RpcProvider,
    ) -> Result<Self, EthPricesError> {
        let network_id = NetworkId::from_provider(&provider).await?;
        let block_number = provider.get_block_number().await?;
        let time = NetworkTime::from_provider(provider, network_id.clone(), block_number);

        self.0.insert(network_id.into(), time);
        Ok(self)
    }
}
