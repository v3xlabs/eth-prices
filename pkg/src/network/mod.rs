/*!
Network timekeeping for multi-chain quoting.

`eth-prices` splits network state into two complementary types:

- [`NetworkTime`] — a single point-in-time on one network (EVM block or fiat timestamp).
- [`NetworkInstant`] — a snapshot of multiple network times, keyed by [`NetworkId`].

Because a single route can hop across different chains (e.g. swap on Ethereum, bridge to
Arbitrum, swap again), the `Quoter::rate` method always receives a
[`NetworkInstant`] so each step can look up the correct provider and block height.

# Construction

There are two entry points:

**1. From a single network** — use [`NetworkTime::instant()`]:

```rust,ignore
use eth_prices::network::NetworkTime;
use alloy::primitives::BlockNumber;

let network = NetworkTime::EVM(1, BlockNumber::from(20_000_000), provider).instant();
```

**2. From scratch using the builder pattern** — chain multiple
[`NetworkInstant`] builder methods together:

```rust,ignore
use eth_prices::network::NetworkInstant;

let networks = NetworkInstant::default()
    .with_fiat_timestamp(1_700_000_000)
    .with_evm_block(1, 20_000_000, eth_provider)
    .with_evm_block(42161, 200_000_000, arb_provider);
```

The async builders `with_evm_latest` and `with_evm_provider` fetch the latest block
height from the RPC before inserting:

```rust,ignore
let networks = NetworkInstant::default()
    .with_evm_latest(1, eth_provider)
    .await?
    .with_fiat_timestamp(now);
```

# Reuse

[`NetworkInstant`] is cheaply clonable. Construct one at the start of your request
and pass a reference to every [`Route::quote`](crate::router::Route::quote) call to
ensure all sub-quotes use the same block heights and providers.
*/

pub mod instant;
pub mod time;

use alloy::providers::Provider;
pub use instant::NetworkInstant;
pub use time::NetworkTime;

use crate::{EthPricesError, provider::RpcProvider};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Network {
    EVM(NetworkId),
    Fiat,
}

impl Network {
    pub async fn from_provider(provider: &RpcProvider) -> Result<Self, EthPricesError> {
        NetworkId::from_provider(provider).await.map(Network::EVM)
    }
}

impl From<NetworkId> for Network {
    fn from(network_id: NetworkId) -> Self {
        Network::EVM(network_id)
    }
}

impl From<&Network> for NetworkId {
    fn from(network: &Network) -> Self {
        match network {
            Network::EVM(network_id) => network_id.clone(),
            Network::Fiat => NetworkId(0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkId(pub u64);

impl NetworkId {
    pub async fn from_provider(provider: &RpcProvider) -> Result<Self, EthPricesError> {
        provider
            .get_chain_id()
            .await
            .map(NetworkId)
            .map_err(EthPricesError::from)
    }
}

impl From<&NetworkId> for Network {
    fn from(network_id: &NetworkId) -> Self {
        Network::EVM(network_id.clone())
    }
}

impl From<u64> for NetworkId {
    fn from(network_id: u64) -> Self {
        NetworkId(network_id)
    }
}
