/*! JS compatibility layer using wasm-bindgen. */

use std::sync::Arc;

use alloy::{
    primitives::{Address, BlockNumber, U256},
    providers::{DynProvider, Provider, ProviderBuilder},
};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{
    quoter::{
        Quoter as QuoterTrait, QuoterInstance,
        erc4626::ERC4626Quoter,
        fixed::FixedQuoter,
        uniswap_v2::{UniswapV2Quoter, UniswapV2Selector},
        uniswap_v3::{UniswapV3Quoter, factory::UniswapV3Selector},
    },
    router::{Route as RouterRoute, graph::QuoterGraph},
    token::TokenIdentifier,
};

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface FixedQuoterConfig {
  token_in: string;
  token_out: string;
  fixed_rate: number;
}

export type UniswapV2Selector =
  | { pair_address: string }
  | { token_in: string; token_out: string };

export type UniswapV3Selector =
  | { pool_address: string }
  | { token_in: string; token_out: string; fee?: number };

export interface QuotersConfig {
  fixed?: FixedQuoterConfig[];
  uniswap_v2?: UniswapV2Selector[];
  uniswap_v3?: UniswapV3Selector[];
  erc4626?: string[];
}

export interface CreateQuoterConfig {
  rpcUrl: string;
  quoters?: QuotersConfig;
}

export interface QuoteRequest {
  inputToken: string;
  outputToken: string;
  amountIn: string;
  block?: bigint;
}

export interface RouteStepView {
  quoterId: string;
  direction: "Forward" | "Reverse";
}

export interface RouteView {
  inputToken: string;
  outputToken: string;
  path: RouteStepView[];
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "FixedQuoterConfig")]
    pub type JsFixedQuoterConfig;

    #[wasm_bindgen(typescript_type = "UniswapV2Selector")]
    pub type JsUniswapV2Selector;

    #[wasm_bindgen(typescript_type = "UniswapV3Selector")]
    pub type JsUniswapV3Selector;

    #[wasm_bindgen(typescript_type = "CreateQuoterConfig")]
    pub type JsCreateQuoterConfig;

    #[wasm_bindgen(typescript_type = "QuoteRequest")]
    pub type JsQuoteRequest;

    #[wasm_bindgen(typescript_type = "RouteView")]
    pub type JsRouteView;
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateQuoterConfig {
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
struct RouteStepView {
    quoter_id: String,
    direction: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RouteView {
    input_token: String,
    output_token: String,
    path: Vec<RouteStepView>,
}

#[wasm_bindgen]
pub struct Quoter {
    provider: DynProvider,
    router: QuoterGraph,
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct Route {
    route: RouterRoute,
}

#[wasm_bindgen]
impl Route {
    #[wasm_bindgen(js_name = inputToken)]
    pub fn input_token(&self) -> String {
        self.route.input_token.to_string()
    }

    #[wasm_bindgen(js_name = outputToken)]
    pub fn output_token(&self) -> String {
        self.route.output_token.to_string()
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsRouteView, JsError> {
        let view = RouteView {
            input_token: self.route.input_token.to_string(),
            output_token: self.route.output_token.to_string(),
            path: self
                .route
                .path
                .iter()
                .map(|step| RouteStepView {
                    quoter_id: step.quoter.id(),
                    direction: format!("{:?}", step.direction),
                })
                .collect(),
        };

        serde_wasm_bindgen::to_value(&view)
            .map(Into::into)
            .map_err(into_js_error)
    }
}

#[wasm_bindgen]
impl Quoter {
    #[wasm_bindgen(js_name = addFixedQuoter)]
    pub fn add_fixed_quoter(&mut self, config: JsFixedQuoterConfig) -> Result<(), JsError> {
        let quoter: FixedQuoter =
            serde_wasm_bindgen::from_value(config.into()).map_err(into_js_error)?;
        self.add_quoter(QuoterInstance::Fixed(quoter));
        Ok(())
    }

    #[wasm_bindgen(js_name = addUniswapV2Quoter)]
    pub async fn add_uniswap_v2_quoter(
        &mut self,
        selector: JsUniswapV2Selector,
    ) -> Result<(), JsError> {
        let selector: UniswapV2Selector =
            serde_wasm_bindgen::from_value(selector.into()).map_err(into_js_error)?;
        let quoter = UniswapV2Quoter::from_selector(&self.provider, selector).await;
        self.add_quoter(QuoterInstance::UniswapV2(quoter));
        Ok(())
    }

    #[wasm_bindgen(js_name = addUniswapV3Quoter)]
    pub async fn add_uniswap_v3_quoter(
        &mut self,
        selector: JsUniswapV3Selector,
    ) -> Result<(), JsError> {
        let selector: UniswapV3Selector =
            serde_wasm_bindgen::from_value(selector.into()).map_err(into_js_error)?;
        let quoter = UniswapV3Quoter::from_selector(&self.provider, selector).await;
        self.add_quoter(QuoterInstance::UniswapV3(quoter));
        Ok(())
    }

    #[wasm_bindgen(js_name = addErc4626Quoter)]
    pub async fn add_erc4626_quoter(&mut self, vault_address: String) -> Result<(), JsError> {
        let vault_address = vault_address
            .parse::<Address>()
            .map_err(|err| into_js_error(anyhow!(err)))?;
        let quoter = ERC4626Quoter::new(vault_address, &self.provider).await;
        self.add_quoter(QuoterInstance::ERC4626(quoter));
        Ok(())
    }

    #[wasm_bindgen(js_name = computeRoute)]
    pub fn compute_route(
        &self,
        input_token: String,
        output_token: String,
    ) -> Result<Route, JsError> {
        let input_token = parse_token_identifier(&input_token)?;
        let output_token = parse_token_identifier(&output_token)?;
        let route = self
            .router
            .compute(&input_token, &output_token)
            .map_err(into_js_error)?;

        Ok(Route { route })
    }

    #[wasm_bindgen(js_name = quoteRoute)]
    pub async fn quote_route(
        &self,
        route: &Route,
        amount_in: String,
        block: Option<u64>,
    ) -> Result<String, JsError> {
        let amount_in = parse_u256(&amount_in)?;
        let block = self.resolve_block(block).await?;
        let amount_out = route
            .route
            .quote(block, amount_in)
            .await
            .map_err(into_js_error)?;
        Ok(amount_out.to_string())
    }

    #[wasm_bindgen(js_name = getRate)]
    pub async fn get_rate(
        &self,
        input_token: String,
        output_token: String,
        amount_in: String,
        block: Option<u64>,
    ) -> Result<String, JsError> {
        let route = self.compute_route(input_token, output_token)?;
        self.quote_route(&route, amount_in, block).await
    }

    #[wasm_bindgen(js_name = quote)]
    pub async fn quote(&self, request: JsQuoteRequest) -> Result<String, JsError> {
        let request: QuoteRequest =
            serde_wasm_bindgen::from_value(request.into()).map_err(into_js_error)?;
        self.get_rate(
            request.input_token,
            request.output_token,
            request.amount_in,
            request.block,
        )
        .await
    }

    #[wasm_bindgen(js_name = getLatestBlock)]
    pub async fn get_latest_block(&self) -> Result<u64, JsError> {
        self.provider
            .get_block_number()
            .await
            .map_err(into_js_error)
    }

    #[wasm_bindgen(js_name = listQuoters)]
    pub fn list_quoters(&self) -> Vec<String> {
        self.router
            .quoters
            .iter()
            .map(|quoter| quoter.id())
            .collect()
    }
}

impl Quoter {
    fn add_quoter(&mut self, quoter: QuoterInstance) {
        self.router.add_quoter(&quoter);
        self.router.quoters.push(Arc::new(quoter));
    }

    async fn resolve_block(&self, block: Option<u64>) -> Result<BlockNumber, JsError> {
        match block {
            Some(block) => Ok(block),
            None => self
                .provider
                .get_block_number()
                .await
                .map_err(into_js_error),
        }
    }
}

#[wasm_bindgen(js_name = createQuoter)]
pub async fn create_quoter(config: JsCreateQuoterConfig) -> Result<Quoter, JsError> {
    let config: CreateQuoterConfig =
        serde_wasm_bindgen::from_value(config.into()).map_err(into_js_error)?;
    let provider = ProviderBuilder::new()
        .connect(&config.rpc_url)
        .await
        .map_err(into_js_error)?
        .erased();

    let mut quoter = Quoter {
        provider,
        router: QuoterGraph::default(),
    };

    for fixed in config.quoters.fixed {
        quoter.add_quoter(QuoterInstance::Fixed(fixed));
    }

    for selector in config.quoters.uniswap_v2 {
        let next = UniswapV2Quoter::from_selector(&quoter.provider, selector).await;
        quoter.add_quoter(QuoterInstance::UniswapV2(next));
    }

    for selector in config.quoters.uniswap_v3 {
        let next = UniswapV3Quoter::from_selector(&quoter.provider, selector).await;
        quoter.add_quoter(QuoterInstance::UniswapV3(next));
    }

    for vault_address in config.quoters.erc4626 {
        let next = ERC4626Quoter::new(vault_address, &quoter.provider).await;
        quoter.add_quoter(QuoterInstance::ERC4626(next));
    }

    Ok(quoter)
}

fn parse_token_identifier(token: &str) -> Result<TokenIdentifier, JsError> {
    TokenIdentifier::try_from(token.to_string()).map_err(into_js_error)
}

fn parse_u256(amount: &str) -> Result<U256, JsError> {
    amount
        .parse::<U256>()
        .map_err(|err| into_js_error(anyhow!(err)))
}

fn into_js_error(err: impl Into<anyhow::Error>) -> JsError {
    JsError::new(&err.into().to_string())
}
