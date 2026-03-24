use std::{collections::BTreeMap, str::FromStr, sync::Arc};

use alloy::{
    primitives::{Address, U256},
    providers::{DynProvider, Provider, ProviderBuilder},
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{
    config::Config,
    quoter::{
        Quoter, QuoterInstance, RateDirection,
        erc4626::ERC4626Quoter,
        fixed::FixedQuoter,
        uniswap_v2::{UniswapV2Quoter, UniswapV2Selector, factory::fetch_pair, pair::fetch_pair_info},
        uniswap_v3::{
            UniswapV3Quoter,
            factory::UniswapV3Selector,
        },
    },
    router::{Route, graph::QuoterGraph},
    token::{Token, TokenIdentifier},
};

#[derive(Serialize)]
struct JsTokenInfo {
    identifier: String,
    name: String,
    symbol: String,
    decimals: u8,
}

#[derive(Serialize)]
struct JsQuoterInfo {
    id: String,
    token_in: String,
    token_out: String,
}

#[derive(Serialize)]
struct JsRouteStep {
    quoter_id: String,
    direction: String,
}

#[derive(Serialize)]
struct JsChainInfo {
    chain_id: u64,
    rpc_url: String,
    token_count: usize,
    fixed_quoters: usize,
    uniswap_v2_pairs: usize,
    uniswap_v3_pools: usize,
    erc4626_quoters: usize,
}

#[derive(Serialize)]
struct JsV2PairInfo {
    reserve0: String,
    reserve1: String,
    block_timestamp_last: u32,
    price0: String,
    price1: String,
    k_last: String,
    token_a: String,
    token_b: String,
}

fn js_err<E: std::fmt::Display>(error: E) -> JsError {
    JsError::new(&error.to_string())
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value.serialize(&serializer).map_err(js_err)
}

fn parse_token_identifier(value: &str) -> Result<TokenIdentifier, JsError> {
    TokenIdentifier::try_from(value.to_string()).map_err(js_err)
}

fn parse_address(value: &str) -> Result<Address, JsError> {
    value.parse::<Address>().map_err(js_err)
}

fn parse_amount(value: &str) -> Result<U256, JsError> {
    U256::from_str(value).map_err(js_err)
}

fn parse_direction(direction: &str) -> Result<RateDirection, JsError> {
    match direction.to_ascii_lowercase().as_str() {
        "forward" => Ok(RateDirection::Forward),
        "reverse" => Ok(RateDirection::Reverse),
        _ => Err(JsError::new("Direction must be 'forward' or 'reverse'")),
    }
}

fn quoter_info(quoter: &QuoterInstance) -> JsQuoterInfo {
    let (token_in, token_out) = quoter.tokens();
    JsQuoterInfo {
        id: quoter.id(),
        token_in: token_in.to_string(),
        token_out: token_out.to_string(),
    }
}

#[wasm_bindgen]
pub fn eth_prices_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[wasm_bindgen]
pub struct EthPrices {
    provider: DynProvider,
}

#[wasm_bindgen]
impl EthPrices {
    #[wasm_bindgen(js_name = connect)]
    pub async fn connect(rpc_url: String) -> Result<EthPrices, JsError> {
        let provider = ProviderBuilder::new()
            .connect(&rpc_url)
            .await
            .map_err(js_err)?;
        Ok(Self {
            provider: provider.erased(),
        })
    }

    #[wasm_bindgen(js_name = parseConfigToml)]
    pub fn parse_config_toml(toml: String) -> Result<JsValue, JsError> {
        let config = Config::from_toml_str(&toml).map_err(js_err)?;
        let mut chains = BTreeMap::new();

        for (chain_slug, chain) in config.chains {
            chains.insert(
                chain_slug,
                JsChainInfo {
                    chain_id: chain.chain_id,
                    rpc_url: chain.rpc_url,
                    token_count: chain.tokens.len(),
                    fixed_quoters: chain.quoters.fixed.len(),
                    uniswap_v2_pairs: chain
                        .quoters
                        .uniswap_v2
                        .as_ref()
                        .map(|x| x.pairs.len())
                        .unwrap_or(0),
                    uniswap_v3_pools: chain
                        .quoters
                        .uniswap_v3
                        .as_ref()
                        .map(|x| x.pools.len())
                        .unwrap_or(0),
                    erc4626_quoters: chain.quoters.erc4626.len(),
                },
            );
        }

        to_js(&chains)
    }

    #[wasm_bindgen(js_name = graphFromToml)]
    pub async fn graph_from_toml(&self, toml: String, chain_slug: String) -> Result<WasmGraph, JsError> {
        let config = Config::from_toml_str(&toml).map_err(js_err)?;
        let chain = config
            .chains
            .get(&chain_slug)
            .ok_or_else(|| JsError::new("Chain slug not found in config"))?;
        let quoters = chain.quoters.all(&self.provider).await;

        Ok(WasmGraph {
            inner: QuoterGraph::from_iter(quoters),
        })
    }

    #[wasm_bindgen(js_name = fixedQuoter)]
    pub fn fixed_quoter(
        &self,
        token_in: String,
        token_out: String,
        fixed_rate: f64,
    ) -> Result<WasmQuoter, JsError> {
        let token_in = parse_token_identifier(&token_in)?;
        let token_out = parse_token_identifier(&token_out)?;

        Ok(WasmQuoter {
            inner: QuoterInstance::Fixed(FixedQuoter {
                token_in,
                token_out,
                fixed_rate,
            }),
        })
    }

    #[wasm_bindgen(js_name = uniswapV2QuoterByPair)]
    pub async fn uniswap_v2_quoter_by_pair(&self, pair_address: String) -> Result<WasmQuoter, JsError> {
        let pair_address = parse_address(&pair_address)?;
        let selector = UniswapV2Selector::Pair { pair_address };
        let quoter = UniswapV2Quoter::from_selector(&self.provider, selector).await;
        Ok(WasmQuoter {
            inner: QuoterInstance::UniswapV2(quoter),
        })
    }

    #[wasm_bindgen(js_name = uniswapV2QuoterByTokens)]
    pub async fn uniswap_v2_quoter_by_tokens(
        &self,
        token_in: String,
        token_out: String,
    ) -> Result<WasmQuoter, JsError> {
        let token_in = parse_address(&token_in)?;
        let token_out = parse_address(&token_out)?;
        let selector = UniswapV2Selector::ByTokens {
            token_in,
            token_out,
        };
        let quoter = UniswapV2Quoter::from_selector(&self.provider, selector).await;
        Ok(WasmQuoter {
            inner: QuoterInstance::UniswapV2(quoter),
        })
    }

    #[wasm_bindgen(js_name = uniswapV2Pair)]
    pub async fn uniswap_v2_pair(
        &self,
        factory_address: String,
        token_in: String,
        token_out: String,
    ) -> Result<String, JsError> {
        let factory_address = parse_address(&factory_address)?;
        let token_in = parse_address(&token_in)?;
        let token_out = parse_address(&token_out)?;
        let pair = fetch_pair(&self.provider, factory_address, token_in, token_out)
            .await
            .map_err(js_err)?;
        Ok(pair.to_string())
    }

    #[wasm_bindgen(js_name = uniswapV2PairInfo)]
    pub async fn uniswap_v2_pair_info(&self, pair_address: String) -> Result<JsValue, JsError> {
        let pair_address = parse_address(&pair_address)?;
        let pair_info = fetch_pair_info(&self.provider, pair_address)
            .await
            .map_err(js_err)?;
        let result = JsV2PairInfo {
            reserve0: pair_info.reserves.0.to_string(),
            reserve1: pair_info.reserves.1.to_string(),
            block_timestamp_last: pair_info.reserves.2,
            price0: pair_info.price0.to_string(),
            price1: pair_info.price1.to_string(),
            k_last: pair_info.k_last.to_string(),
            token_a: pair_info.token_a.to_string(),
            token_b: pair_info.token_b.to_string(),
        };
        to_js(&result)
    }

    #[wasm_bindgen(js_name = uniswapV3PoolByTokens)]
    pub async fn uniswap_v3_pool_by_tokens(
        &self,
        token_in: String,
        token_out: String,
        fee: Option<u32>,
    ) -> Result<String, JsError> {
        let token_in = parse_address(&token_in)?;
        let token_out = parse_address(&token_out)?;
        let selector = UniswapV3Selector::ByTokens {
            token_in,
            token_out,
            fee,
        };
        let pool = selector.resolve(&self.provider).await.map_err(js_err)?;
        Ok(pool.to_string())
    }

    #[wasm_bindgen(js_name = uniswapV3QuoterByPool)]
    pub async fn uniswap_v3_quoter_by_pool(&self, pool_address: String) -> Result<WasmQuoter, JsError> {
        let pool_address = parse_address(&pool_address)?;
        let selector = UniswapV3Selector::Pool { pool_address };
        let quoter = UniswapV3Quoter::from_selector(&self.provider, selector).await;
        Ok(WasmQuoter {
            inner: QuoterInstance::UniswapV3(quoter),
        })
    }

    #[wasm_bindgen(js_name = uniswapV3QuoterByTokens)]
    pub async fn uniswap_v3_quoter_by_tokens(
        &self,
        token_in: String,
        token_out: String,
        fee: Option<u32>,
    ) -> Result<WasmQuoter, JsError> {
        let token_in = parse_address(&token_in)?;
        let token_out = parse_address(&token_out)?;
        let selector = UniswapV3Selector::ByTokens {
            token_in,
            token_out,
            fee,
        };
        let quoter = UniswapV3Quoter::from_selector(&self.provider, selector).await;
        Ok(WasmQuoter {
            inner: QuoterInstance::UniswapV3(quoter),
        })
    }

    #[wasm_bindgen(js_name = erc4626Quoter)]
    pub async fn erc4626_quoter(&self, vault_address: String) -> Result<WasmQuoter, JsError> {
        let vault_address = parse_address(&vault_address)?;
        let quoter = ERC4626Quoter::new(vault_address, &self.provider).await;
        Ok(WasmQuoter {
            inner: QuoterInstance::ERC4626(quoter),
        })
    }

    #[wasm_bindgen(js_name = tokenInfo)]
    pub async fn token_info(&self, token_identifier: String) -> Result<JsValue, JsError> {
        let identifier = parse_token_identifier(&token_identifier)?;
        let token = Token::new(identifier.clone(), &self.provider)
            .await
            .map_err(js_err)?;

        to_js(&JsTokenInfo {
            identifier: identifier.to_string(),
            name: token.name,
            symbol: token.symbol,
            decimals: token.decimals,
        })
    }

    #[wasm_bindgen(js_name = tokenNominalAmount)]
    pub async fn token_nominal_amount(&self, token_identifier: String) -> Result<String, JsError> {
        let identifier = parse_token_identifier(&token_identifier)?;
        let token = Token::new(identifier, &self.provider).await.map_err(js_err)?;
        Ok(token.nominal_amount().await.to_string())
    }

    #[wasm_bindgen(js_name = tokenFormatAmount)]
    pub async fn token_format_amount(
        &self,
        token_identifier: String,
        amount: String,
        precision: usize,
    ) -> Result<String, JsError> {
        let identifier = parse_token_identifier(&token_identifier)?;
        let amount = parse_amount(&amount)?;
        let token = Token::new(identifier, &self.provider).await.map_err(js_err)?;
        Ok(token.format_amount(amount, precision).await)
    }
}

#[wasm_bindgen]
pub struct WasmQuoter {
    inner: QuoterInstance,
}

#[wasm_bindgen]
impl WasmQuoter {
    pub fn id(&self) -> String {
        self.inner.id()
    }

    pub fn tokens(&self) -> Result<JsValue, JsError> {
        to_js(&quoter_info(&self.inner))
    }

    pub async fn rate(
        &self,
        amount_in: String,
        direction: String,
        block: u32,
    ) -> Result<String, JsError> {
        let amount_in = parse_amount(&amount_in)?;
        let direction = parse_direction(&direction)?;
        let amount_out = self
            .inner
            .rate(amount_in, direction, block as u64)
            .await
            .map_err(js_err)?;
        Ok(amount_out.to_string())
    }
}

#[wasm_bindgen]
pub struct WasmGraph {
    inner: QuoterGraph,
}

#[wasm_bindgen]
impl WasmGraph {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmGraph {
        Self {
            inner: QuoterGraph::default(),
        }
    }

    #[wasm_bindgen(js_name = addQuoter)]
    pub fn add_quoter(&mut self, quoter: &WasmQuoter) {
        self.inner.add_quoter(&quoter.inner);
        self.inner.quoters.push(Arc::new(quoter.inner.clone()));
    }

    #[wasm_bindgen(js_name = quoterCount)]
    pub fn quoter_count(&self) -> usize {
        self.inner.quoters.len()
    }

    pub fn quoters(&self) -> Result<JsValue, JsError> {
        let list: Vec<JsQuoterInfo> = self
            .inner
            .quoters
            .iter()
            .map(|quoter| quoter_info(quoter))
            .collect();
        to_js(&list)
    }

    #[wasm_bindgen(js_name = toGraphviz)]
    pub fn to_graphviz(&self) -> String {
        self.inner.to_graphviz()
    }

    pub fn compute(
        &self,
        input_token: String,
        output_token: String,
    ) -> Result<WasmRoute, JsError> {
        let input_token = parse_token_identifier(&input_token)?;
        let output_token = parse_token_identifier(&output_token)?;
        let route = self
            .inner
            .compute(&input_token, &output_token)
            .map_err(js_err)?;

        Ok(WasmRoute { inner: route })
    }
}

#[wasm_bindgen]
pub struct WasmRoute {
    inner: Route,
}

#[wasm_bindgen]
impl WasmRoute {
    #[wasm_bindgen(js_name = inputToken)]
    pub fn input_token(&self) -> String {
        self.inner.input_token.to_string()
    }

    #[wasm_bindgen(js_name = outputToken)]
    pub fn output_token(&self) -> String {
        self.inner.output_token.to_string()
    }

    pub fn steps(&self) -> Result<JsValue, JsError> {
        let steps: Vec<JsRouteStep> = self
            .inner
            .path
            .iter()
            .map(|step| JsRouteStep {
                quoter_id: step.quoter.id(),
                direction: step.direction.to_string(),
            })
            .collect();
        to_js(&steps)
    }

    pub async fn quote(&self, block: u32, amount_in: String) -> Result<String, JsError> {
        let amount_in = parse_amount(&amount_in)?;
        let amount_out = self
            .inner
            .quote(block as u64, amount_in)
            .await
            .map_err(js_err)?;
        Ok(amount_out.to_string())
    }
}
