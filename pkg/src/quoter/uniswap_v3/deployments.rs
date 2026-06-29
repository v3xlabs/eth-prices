use alloy::primitives::{Address, address};

/// Deployment metadata for a Uniswap V3-compatible factory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UniswapV3Deployment {
    /// EVM chain ID.
    pub chain_id: u64,
    /// Exchange name (e.g. "Uniswap", "SushiSwap", "PancakeSwap").
    pub name: &'static str,
    /// Factory contract address.
    pub factory: Address,
}

/// All known V3 factory deployments.
pub const DEPLOYMENTS: &[UniswapV3Deployment] = &[
    // ── Uniswap V3 ─────────────────────────────────────────────────────
    UniswapV3Deployment {
        chain_id: 1,
        name: "Uniswap",
        factory: address!("0x1F98431c8aD98523631AE4a59f267346ea31F984"),
    },
    // ── SushiSwap clAMM (V3 fork) ──────────────────────────────────────
    UniswapV3Deployment {
        chain_id: 1,
        name: "SushiSwap",
        factory: address!("0xbaceb8ec6b9355dfc0269c18bac9d6e2bdc29c4f"),
    },
    UniswapV3Deployment {
        chain_id: 10,
        name: "SushiSwap",
        factory: address!("0x9c6522117e2ed1fe5bdb72bb0ed5e3f2bde7dbe0"),
    },
    UniswapV3Deployment {
        chain_id: 56,
        name: "SushiSwap",
        factory: address!("0x126555dd55a39328f69400d6ae4f782bd4c34abb"),
    },
    UniswapV3Deployment {
        chain_id: 100,
        name: "SushiSwap",
        factory: address!("0xf78031cbca409f2fb6876bdfdbc1b2df24cf9bef"),
    },
    UniswapV3Deployment {
        chain_id: 137,
        name: "SushiSwap",
        factory: address!("0x917933899c6a5f8e37f31e19f92cdbff7e8ff0e2"),
    },
    UniswapV3Deployment {
        chain_id: 250,
        name: "SushiSwap",
        factory: address!("0x7770978eed668a3ba661d51a773d3a992fc9ddcb"),
    },
    UniswapV3Deployment {
        chain_id: 1088,
        name: "SushiSwap",
        factory: address!("0x145d82bca93cca2ae057d1c6f26245d1b9522e6f"),
    },
    UniswapV3Deployment {
        chain_id: 1101,
        name: "SushiSwap",
        factory: address!("0x1b02da8cb0d097eb8d57a175b88c7d8b47997506"),
    },
    UniswapV3Deployment {
        chain_id: 8453,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV3Deployment {
        chain_id: 42161,
        name: "SushiSwap",
        factory: address!("0x1af415a1eba07a4986a52b6f2e7de7003d82231e"),
    },
    UniswapV3Deployment {
        chain_id: 43114,
        name: "SushiSwap",
        factory: address!("0x3e603c14af37ebdad31709c4f848fc6ad5bec715"),
    },
    UniswapV3Deployment {
        chain_id: 59144,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV3Deployment {
        chain_id: 81457,
        name: "SushiSwap",
        factory: address!("0x7680d4b43f3d1d54d6cfeeb2169463bfa7a6cf0d"),
    },
    UniswapV3Deployment {
        chain_id: 534352,
        name: "SushiSwap",
        factory: address!("0x46b3fdf7b5cde91ac049936bf0bdb12c5d22202e"),
    },
    UniswapV3Deployment {
        chain_id: 146,
        name: "SushiSwap",
        factory: address!("0x46b3fdf7b5cde91ac049936bf0bdb12c5d22202e"),
    },
    UniswapV3Deployment {
        chain_id: 30,
        name: "SushiSwap",
        factory: address!("0x46b3fdf7b5cde91ac049936bf0bdb12c5d22202e"),
    },
    UniswapV3Deployment {
        chain_id: 122,
        name: "SushiSwap",
        factory: address!("0x1b9d177ccdea3c79b6c8f40761fc8dc9d0500eaa"),
    },
    UniswapV3Deployment {
        chain_id: 199,
        name: "SushiSwap",
        factory: address!("0xbbde1d67297329148fe1ed5e6b00114842728e65"),
    },
    UniswapV3Deployment {
        chain_id: 288,
        name: "SushiSwap",
        factory: address!("0x0be808376ecb75a5cf9bb6d237d16cd37893d904"),
    },
    UniswapV3Deployment {
        chain_id: 314,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV3Deployment {
        chain_id: 1116,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV3Deployment {
        chain_id: 7000,
        name: "SushiSwap",
        factory: address!("0xb45e53277a7e0f1d35f2a77160e91e25507f1763"),
    },
    UniswapV3Deployment {
        chain_id: 11235,
        name: "SushiSwap",
        factory: address!("0xc35dadb65012ec5796536bd9864ed8773abc74c4"),
    },
    UniswapV3Deployment {
        chain_id: 11155111,
        name: "SushiSwap",
        factory: address!("0x1f2fcf1d036b375b384012e61d3aa33f8c256bbe"),
    },
    // ── PancakeSwap V3 ─────────────────────────────────────────────────
    UniswapV3Deployment {
        chain_id: 1,
        name: "PancakeSwap",
        factory: address!("0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865"),
    },
    UniswapV3Deployment {
        chain_id: 56,
        name: "PancakeSwap",
        factory: address!("0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865"),
    },
    UniswapV3Deployment {
        chain_id: 204,
        name: "PancakeSwap",
        factory: address!("0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865"),
    },
    UniswapV3Deployment {
        chain_id: 324,
        name: "PancakeSwap",
        factory: address!("0x1BB72E0CbbEA93c08f535fc7856E0338D7F7a8aB"),
    },
    UniswapV3Deployment {
        chain_id: 42161,
        name: "PancakeSwap",
        factory: address!("0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865"),
    },
    UniswapV3Deployment {
        chain_id: 59144,
        name: "PancakeSwap",
        factory: address!("0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865"),
    },
    UniswapV3Deployment {
        chain_id: 8453,
        name: "PancakeSwap",
        factory: address!("0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865"),
    },
];

/// Returns all V3 deployments for the given chain.
pub fn deployments_for_chain(chain_id: u64) -> impl Iterator<Item = &'static UniswapV3Deployment> {
    DEPLOYMENTS.iter().filter(move |d| d.chain_id == chain_id)
}

/// Returns all V3 factory addresses for the given chain.
pub fn factories_for_chain(chain_id: u64) -> impl Iterator<Item = Address> {
    deployments_for_chain(chain_id).map(|d| d.factory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ethereum_has_three_v3_dexes() {
        let addrs: Vec<Address> = factories_for_chain(1).collect();
        assert!(addrs.contains(&address!("0x1F98431c8aD98523631AE4a59f267346ea31F984")));
        assert!(addrs.contains(&address!("0xbaceb8ec6b9355dfc0269c18bac9d6e2bdc29c4f")));
        assert!(addrs.contains(&address!("0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865")));
    }

    #[test]
    fn test_bsc_has_v3_dexes() {
        let addrs: Vec<Address> = factories_for_chain(56).collect();
        assert!(!addrs.is_empty());
    }

    #[test]
    fn test_unknown_chain_is_empty() {
        let addrs: Vec<Address> = factories_for_chain(999999).collect();
        assert!(addrs.is_empty());
    }
}
