use alloy::{eips::BlockId, primitives::Address};
use futures::future::join_all;

use crate::{
    asset::identity::AssetIdentifier,
    network::NetworkId,
    provider::RpcProvider,
    quoter::{
        AnyQuoter,
        erc4626::{ERC4626, ERC4626Quoter},
    },
    router::discovery::{DiscovererReport, DiscoveryFailure},
};

const VAULT_CONFIDENCE: u64 = 50;

/// Probes every address for an ERC-4626 `asset()` and returns a quoter per
/// vault together with the underlying assets it uncovered. Probe errors are
/// expected for plain ERC-20 tokens and are reported as failures for parity
/// with the TypeScript discoverer.
pub async fn discover_quoters(
    provider: &RpcProvider,
    network_id: &NetworkId,
    addresses: Vec<Address>,
    block: BlockId,
) -> (Vec<AnyQuoter>, Vec<Address>, DiscovererReport) {
    let attempted = addresses.len();

    let results: Vec<_> = join_all(addresses.into_iter().map(|addr| {
        let provider = provider.clone();
        let net_id = network_id.clone();
        async move {
            match ERC4626::new(addr, &provider)
                .asset()
                .block(block)
                .call()
                .await
            {
                Ok(underlying) => {
                    let quoter = ERC4626Quoter {
                        network_id: net_id,
                        vault_address: AssetIdentifier::ERC20 { address: addr },
                        token_address: AssetIdentifier::ERC20 {
                            address: underlying,
                        },
                    };
                    Ok((
                        AnyQuoter::from(quoter).with_confidence(VAULT_CONFIDENCE),
                        underlying,
                    ))
                }
                Err(error) => Err(DiscoveryFailure {
                    target: addr.to_string(),
                    message: error.to_string(),
                }),
            }
        }
    }))
    .await;

    let mut quoters = Vec::new();
    let mut underlying = Vec::new();
    let mut failures = Vec::new();
    for result in results {
        match result {
            Ok((quoter, asset)) => {
                quoters.push(quoter);
                underlying.push(asset);
            }
            Err(failure) => failures.push(failure),
        }
    }

    let discovered = quoters.len();
    (
        quoters,
        underlying,
        DiscovererReport {
            identity: format!("erc4626:{}", network_id.0),
            attempted,
            discovered,
            skipped: failures.len(),
            failures,
        },
    )
}
