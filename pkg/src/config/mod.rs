use alloy::{primitives::map::HashMap, providers::DynProvider};
use figment::{
    Figment,
    providers::{Format, Toml},
};
use serde::Deserialize;

use crate::{
    Result,
    error::EthPricesError,
    quoter::{
        AnyQuoter, Quoter,
        erc4626::{ERC4626Config, ERC4626Quoter},
        fixed::FixedQuoter,
        uniswap_v2::{UniswapV2Config, UniswapV2Quoter},
        uniswap_v3::{UniswapV3Quoter, factory::UniswapV3Config},
    },
};

#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    pub chains: HashMap<String, ChainConfig>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub rpc_url: String,
    pub tokens: Vec<TokenConfig>,
    pub quoters: QuotersConfig,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct QuotersConfig {
    pub fixed: Vec<FixedQuoter>,
    pub uniswap_v2: Option<UniswapV2Config>,
    pub uniswap_v3: Option<UniswapV3Config>,
    pub erc4626: Vec<ERC4626Config>,
}

impl QuotersConfig {
    pub async fn all(self, provider: &DynProvider) -> Result<Vec<AnyQuoter>> {
        let mut quoters: Vec<AnyQuoter> = Vec::new();
        for tracker in self.fixed {
            if tracker.fixed_rate <= 0.0 {
                return Err(EthPricesError::InvalidConfiguration(format!(
                    "Fixed rate for {} to {} must be > 0.0",
                    tracker.token_in, tracker.token_out
                )));
            }
            let boxed: Box<dyn Quoter> = Box::new(tracker);
            quoters.push(boxed.strip());
        }

        if let Some(uniswap_v2_config) = &self.uniswap_v2 {
            for uni_quoters in uniswap_v2_config.pairs.iter() {
                let quoter = UniswapV2Quoter::from_selector(provider, uni_quoters.clone()).await?;
                let boxed: Box<dyn Quoter> = Box::new(quoter);
                quoters.push(boxed.strip());
            }
        }

        if let Some(uniswap_v3_config) = &self.uniswap_v3 {
            for uni_quoters in uniswap_v3_config.pools.iter() {
                let quoter = UniswapV3Quoter::from_selector(provider, uni_quoters.clone()).await?;
                let boxed: Box<dyn Quoter> = Box::new(quoter);
                quoters.push(boxed.strip());
            }
        }

        for erc4626_config in &self.erc4626 {
            let quoter = ERC4626Quoter::new(erc4626_config.vault_address, provider).await?;
            let boxed: Box<dyn Quoter> = Box::new(quoter);
            quoters.push(boxed.strip());
        }

        Ok(quoters)
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct TokenConfig {
    pub address: String,
    pub slug: Option<String>,
    pub decimals: u8,
}

impl Config {
    pub async fn load(path: &str) -> Result<Self> {
        let figment = Figment::new().merge(Toml::file(path));
        figment
            .extract::<Config>()
            .map_err(|e| EthPricesError::ConfigError(e.to_string()))
    }
}
