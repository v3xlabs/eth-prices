use alloy::{primitives::{Address, U256}, providers::DynProvider};
use anyhow::Result;

use crate::token::{
    erc20::ERC20,
    identity::TokenIdentifier,
};

pub mod erc20;
pub mod identity;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub identifier: TokenIdentifier,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

const FIAT_DECIMALS: u8 = 6;

impl Token {
    pub async fn new(identifier: TokenIdentifier, provider: &DynProvider) -> Result<Self> {
        let (name, symbol, decimals) = match &identifier {
            TokenIdentifier::ERC20 { address } => {
                let erc20 = ERC20::new(*address, provider);

                (erc20.name().call().await?, erc20.symbol().call().await?, erc20.decimals().call().await?)
            },
            TokenIdentifier::Fiat { symbol } => {

                (symbol.clone(), symbol.clone(), FIAT_DECIMALS)
            },
            TokenIdentifier::Native => {
               ("Native".to_string(), "ETH".to_string(), 18)
            },
        };

        Ok(Self {
            identifier,
            name,
            symbol,
            decimals,
        })
    }

    pub async fn nominal_amount(&self) -> U256 {
        U256::from(10).pow(U256::from(self.decimals))
    }

    pub async fn format_amount(&self, amount: U256, precision: usize) -> String {
        let amount = amount.to_string().parse::<f64>().unwrap();
        let amount = amount / 10_f64.powf(self.decimals as f64);
        format!("{:.precision$}", amount)
    }

    pub fn unwrap_address(&self) -> Address {
        match &self.identifier {
            TokenIdentifier::ERC20 { address } => *address,
            TokenIdentifier::Fiat { symbol: _ } => panic!("Fiat tokens do not have an address"),
            TokenIdentifier::Native => panic!("Native tokens do not have an address"),
        }
    }
}
