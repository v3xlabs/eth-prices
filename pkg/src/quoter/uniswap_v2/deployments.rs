use alloy::primitives::{Address, address};

/// Deployment metadata for a Uniswap V2-compatible factory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UniswapV2Deployment {
    /// EVM chain ID.
    pub chain_id: u64,
    /// Exchange name (e.g. "Uniswap", "SushiSwap", "PancakeSwap").
    pub name: &'static str,
    /// Factory contract address.
    pub factory: Address,
}

/// All known V2 factory deployments.
pub const DEPLOYMENTS: &[UniswapV2Deployment] = &[
    // ── Uniswap V2 ─────────────────────────────────────────────────────
    UniswapV2Deployment {
        chain_id: 1,
        name: "Uniswap",
        factory: address!("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"),
    },
    // ── SushiSwap cpAMM (V2 fork) ──────────────────────────────────────
    UniswapV2Deployment {
        chain_id: 1,
        name: "SushiSwap",
        factory: address!("0xc0aee478e3658e2610c5f7a4a2e1777ce9e4f2ac"),
    },
    UniswapV2Deployment {
        chain_id: 10,
        name: "SushiSwap",
        factory: address!("0xfbc12984689e5f15626bad03ad60160fe98b303c"),
    },
    UniswapV2Deployment {
        chain_id: 56,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV2Deployment {
        chain_id: 100,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV2Deployment {
        chain_id: 137,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV2Deployment {
        chain_id: 250,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV2Deployment {
        chain_id: 288,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV2Deployment {
        chain_id: 1088,
        name: "SushiSwap",
        factory: address!("0x580ed43f3bba06555785c81c2957efcca71f7483"),
    },
    UniswapV2Deployment {
        chain_id: 1101,
        name: "SushiSwap",
        factory: address!("0xb45e53277a7e0f1d35f2a77160e91e25507f1763"),
    },
    UniswapV2Deployment {
        chain_id: 8453,
        name: "SushiSwap",
        factory: address!("0x71524b4f93c58fcbf659783284e38825f0622859"),
    },
    UniswapV2Deployment {
        chain_id: 42161,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV2Deployment {
        chain_id: 43114,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV2Deployment {
        chain_id: 59144,
        name: "SushiSwap",
        factory: address!("0xfbc12984689e5f15626bad03ad60160fe98b303c"),
    },
    UniswapV2Deployment {
        chain_id: 81457,
        name: "SushiSwap",
        factory: address!("0x42fa929fc636e657ac568c0b5cf38e203b67ac2b"),
    },
    UniswapV2Deployment {
        chain_id: 534352,
        name: "SushiSwap",
        factory: address!("0xb45e53277a7e0f1d35f2a77160e91e25507f1763"),
    },
    UniswapV2Deployment {
        chain_id: 146,
        name: "SushiSwap",
        factory: address!("0xb45e53277a7e0f1d35f2a77160e91e25507f1763"),
    },
    UniswapV2Deployment {
        chain_id: 30,
        name: "SushiSwap",
        factory: address!("0xb45e53277a7e0f1d35f2a77160e91e25507f1763"),
    },
    UniswapV2Deployment {
        chain_id: 108,
        name: "SushiSwap",
        factory: address!("0xb45e53277a7e0f1d35f2a77160e91e25507f1763"),
    },
    UniswapV2Deployment {
        chain_id: 122,
        name: "SushiSwap",
        factory: address!("0x43ea90e2b786728520e4f930d2a71a477bf2737c"),
    },
    UniswapV2Deployment {
        chain_id: 199,
        name: "SushiSwap",
        factory: address!("0xb45e53277a7e0f1d35f2a77160e91e25507f1763"),
    },
    UniswapV2Deployment {
        chain_id: 314,
        name: "SushiSwap",
        factory: address!("0x9b3336186a38e1b6c21955d112dbb0343ee061ee"),
    },
    UniswapV2Deployment {
        chain_id: 1116,
        name: "SushiSwap",
        factory: address!("0xb45e53277a7e0f1d35f2a77160e91e25507f1763"),
    },
    UniswapV2Deployment {
        chain_id: 7000,
        name: "SushiSwap",
        factory: address!("0x33d91116e0370970444b0281ab117e161febfcdd"),
    },
    UniswapV2Deployment {
        chain_id: 11235,
        name: "SushiSwap",
        factory: address!("0xb45e53277a7e0f1d35f2a77160e91e25507f1763"),
    },
    UniswapV2Deployment {
        chain_id: 42220,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV2Deployment {
        chain_id: 11155111,
        name: "SushiSwap",
        factory: address!("0x734583f62bb6ace3c9ba9bd5a53143ca2ce8c55a"),
    },
    // ── PancakeSwap V2 ─────────────────────────────────────────────────
    UniswapV2Deployment {
        chain_id: 1,
        name: "PancakeSwap",
        factory: address!("0x1097053Fd2ea711dad45caCcc45EfF7548fCB362"),
    },
    UniswapV2Deployment {
        chain_id: 56,
        name: "PancakeSwap",
        factory: address!("0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73"),
    },
    UniswapV2Deployment {
        chain_id: 324,
        name: "PancakeSwap",
        factory: address!("0xd03D8D566183F0086d8D09A84E1e30b58Dd5619d"),
    },
    UniswapV2Deployment {
        chain_id: 204,
        name: "PancakeSwap",
        factory: address!("0x02a84c1b3BBD7401a5f7fa98a384EBC70bB5749E"),
    },
    UniswapV2Deployment {
        chain_id: 42161,
        name: "PancakeSwap",
        factory: address!("0x02a84c1b3BBD7401a5f7fa98a384EBC70bB5749E"),
    },
    UniswapV2Deployment {
        chain_id: 59144,
        name: "PancakeSwap",
        factory: address!("0x02a84c1b3BBD7401a5f7fa98a384EBC70bB5749E"),
    },
    UniswapV2Deployment {
        chain_id: 8453,
        name: "PancakeSwap",
        factory: address!("0x02a84c1b3BBD7401a5f7fa98a384EBC70bB5749E"),
    },
];

/// Returns all V2 deployments for the given chain.
pub fn deployments_for_chain(chain_id: u64) -> impl Iterator<Item = &'static UniswapV2Deployment> {
    DEPLOYMENTS.iter().filter(move |d| d.chain_id == chain_id)
}

/// Returns all V2 factory addresses for the given chain.
pub fn factories_for_chain(chain_id: u64) -> impl Iterator<Item = Address> {
    deployments_for_chain(chain_id).map(|d| d.factory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ethereum_has_three_v2_dexes() {
        let addrs: Vec<Address> = factories_for_chain(1).collect();
        assert!(addrs.contains(&address!("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f")));
        assert!(addrs.contains(&address!("0xc0aee478e3658e2610c5f7a4a2e1777ce9e4f2ac")));
        assert!(addrs.contains(&address!("0x1097053Fd2ea711dad45caCcc45EfF7548fCB362")));
    }

    #[test]
    fn test_bsc_has_sushi_and_pancake() {
        let addrs: Vec<Address> = factories_for_chain(56).collect();
        assert!(addrs.contains(&address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4")));
        assert!(addrs.contains(&address!("0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73")));
    }

    #[test]
    fn test_unknown_chain_is_empty() {
        let addrs: Vec<Address> = factories_for_chain(999999).collect();
        assert!(addrs.is_empty());
    }
}
