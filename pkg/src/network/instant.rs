use std::collections::HashMap;

use alloy::{primitives::BlockNumber, providers::Provider};

use crate::{
    EthPricesError,
    network::{Network, NetworkId, NetworkTime},
    provider::RpcProvider,
};

/// A snapshot of network states across multiple chains / data sources.
///
/// Internally a `HashMap<Network, NetworkTime>`, where:
///
/// - [`NetworkTime`] stores the actual point-in-time (block height or unix timestamp)
///   together with the provider needed to query it.
///
/// ```rust,ignore
/// use eth_prices::network::NetworkInstant;
///
/// let networks = NetworkInstant::default()
///     .with_evm_provider_latest(eth_provider).await?
///     .with_evm_provider_latest(sep_provider).await?
///     .with_now();
/// ```
///
/// Note that the above example makes a `eth_chainId` and `eth_blockNumber` query per provider supplied.
///  
/// # Quoting at a given network time
///
/// Start with [`Default::default()`] and chain builder methods. Each builder consumes
/// and returns `self`, enabling a fluent style:
///
/// ```rust,ignore
/// use eth_prices::network::NetworkInstant;
///
/// let networks = NetworkInstant::default()
///     .with_evm_block(1.into(), 123_456_789, eth_provider)
///     .with_evm_block(11155111.into(), 123_456_789, sep_provider)
///     .with_fiat_timestamp(time);
/// ```
///
/// For single-network convenience, start with [`NetworkTime::instant()`] instead.
///
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

    /// Set a fiat timestamp to the current time.
    #[cfg(feature = "time")]
    pub fn with_now(mut self) -> Result<Self, EthPricesError> {
        self.0.insert(Network::Fiat, NetworkTime::with_fiat_now());
        Ok(self)
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
