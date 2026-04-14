use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::{
    quoter::{
        RateDirection,
        fixed::FixedQuoter,
        uniswap_v2::UniswapV2Selector,
        uniswap_v3::factory::UniswapV3Selector,
    },
    router::Route as RouterRoute,
};

#[derive(Debug, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct CreateEngineConfig {
    #[serde(default)]
    #[tsify(type = "string")]
    pub rpc_url: Option<String>,
    #[serde(default)]
    pub quoters: QuotersConfig,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "CreateEngineConfig")]
    pub type JsCreateEngineConfig;
}

#[derive(Debug, Deserialize, Tsify, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuotersConfig {
    #[serde(default)]
    pub fixed: Vec<FixedQuoter>,
    #[serde(default)]
    pub uniswap_v2: Vec<UniswapV2Selector>,
    #[serde(default)]
    pub uniswap_v3: Vec<UniswapV3Selector>,
    #[serde(default)]
    #[tsify(type = "string[]")]
    pub erc4626: Vec<Address>,
}

#[derive(Debug, Deserialize, Tsify)]
#[tsify(from_wasm_abi, large_number_types_as_bigints)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    pub input_token: String,
    pub output_token: String,
    pub amount_in: String,
    pub block: Option<u64>,
}

#[derive(Debug, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct RouteStepView {
    pub quoter_id: String,
    #[tsify(type = "\"Forward\" | \"Reverse\"")]
    pub direction: &'static str,
}

#[derive(Debug, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
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
                    quoter_id: step.quoter.to_string(),
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
