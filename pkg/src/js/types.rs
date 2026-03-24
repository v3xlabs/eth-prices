use alloy::primitives::Address;
use serde::{Deserialize, Serialize};

use crate::{
    quoter::{
        Quoter as QuoterTrait, RateDirection, fixed::FixedQuoter, uniswap_v2::UniswapV2Selector,
        uniswap_v3::factory::UniswapV3Selector,
    },
    router::Route as RouterRoute,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEngineConfig {
    pub rpc_url: String,
    #[serde(default)]
    pub quoters: QuotersConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct QuotersConfig {
    #[serde(default)]
    pub fixed: Vec<FixedQuoter>,
    #[serde(default)]
    pub uniswap_v2: Vec<UniswapV2Selector>,
    #[serde(default)]
    pub uniswap_v3: Vec<UniswapV3Selector>,
    #[serde(default)]
    pub erc4626: Vec<Address>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    pub input_token: String,
    pub output_token: String,
    pub amount_in: String,
    pub block: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteStepView {
    pub quoter_id: String,
    pub direction: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteView {
    pub input_token: String,
    pub output_token: String,
    pub path: Vec<RouteStepView>,
}

impl From<&RouterRoute> for RouteView {
    fn from(route: &RouterRoute) -> Self {
        Self {
            input_token: route.input_token.to_string(),
            output_token: route.output_token.to_string(),
            path: route
                .path
                .iter()
                .map(|step| RouteStepView {
                    quoter_id: step.quoter.id(),
                    direction: direction_label(step.direction),
                })
                .collect(),
        }
    }
}

fn direction_label(direction: RateDirection) -> &'static str {
    match direction {
        RateDirection::Forward => "Forward",
        RateDirection::Reverse => "Reverse",
    }
}
