use alloy::{
    primitives::BlockNumber,
    providers::{DynProvider, Provider, ProviderBuilder},
};
use wasm_bindgen::prelude::*;

use super::{
    convert::{into_js_error, parse_address, parse_token_identifier, parse_u256},
    route::Route,
    types::{CreateEngineConfig, JsCreateEngineConfig, QuoteRequest},
};
use crate::{
    Result,
    quoter::{
        AnyQuoter,
        erc4626::ERC4626Quoter,
        fixed::FixedQuoter,
        uniswap_v2::{UniswapV2Quoter, UniswapV2Selector},
        uniswap_v3::{UniswapV3Quoter, factory::UniswapV3Selector},
    },
    router::graph::QuoterGraph,
};

#[wasm_bindgen]
pub struct Engine {
    provider: DynProvider,
    router: QuoterGraph,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(js_name = addFixedQuoter)]
    pub fn add_fixed_quoter(&mut self, quoter: FixedQuoter) -> Result<(), JsError> {
        self.push_quoter(quoter.strip());
        Ok(())
    }

    #[wasm_bindgen(js_name = addUniswapV2Quoter)]
    pub async fn add_uniswap_v2_quoter(
        &mut self,
        selector: UniswapV2Selector,
    ) -> Result<(), JsError> {
        let quoter = UniswapV2Quoter::from_selector(&self.provider, selector)
            .await
            .map_err(into_js_error)?;
        self.push_quoter(quoter.strip());
        Ok(())
    }

    #[wasm_bindgen(js_name = addUniswapV3Quoter)]
    pub async fn add_uniswap_v3_quoter(
        &mut self,
        selector: UniswapV3Selector,
    ) -> Result<(), JsError> {
        let quoter = UniswapV3Quoter::from_selector(&self.provider, selector)
            .await
            .map_err(into_js_error)?;
        self.push_quoter(quoter.strip());
        Ok(())
    }

    #[wasm_bindgen(js_name = addErc4626Quoter)]
    pub async fn add_erc4626_quoter(&mut self, vault_address: String) -> Result<(), JsError> {
        let vault_address = parse_address(&vault_address)?;
        let quoter = ERC4626Quoter::new(vault_address, &self.provider)
            .await
            .map_err(into_js_error)?;
        self.push_quoter(quoter.strip());
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
        self.router
            .compute(&input_token, &output_token)
            .map(Route::from)
            .map_err(into_js_error)
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
        route
            .inner
            .quote(block, amount_in)
            .await
            .map(|amount_out| amount_out.to_string())
            .map_err(into_js_error)
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
    pub async fn quote(&self, request: QuoteRequest) -> Result<String, JsError> {
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
            .map(|quoter| quoter.to_string())
            .collect()
    }
}

impl Engine {
    fn push_quoter(&mut self, quoter: AnyQuoter) {
        self.router.add_quoter(quoter);
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

    async fn from_config(config: CreateEngineConfig) -> Result<Self, JsError> {
        let rpc_url = config.rpc_url.unwrap_or_default();
        if rpc_url.trim().is_empty() {
            return Err(JsError::new("rpcUrl is required"));
        }
        let provider = ProviderBuilder::new()
            .connect(&rpc_url)
            .await
            .map_err(into_js_error)?
            .erased();

        let mut quoter = Self {
            provider,
            router: QuoterGraph::default(),
        };

        quoter.load_fixed(config.quoters.fixed);
        quoter.load_uniswap_v2(config.quoters.uniswap_v2).await?;
        quoter.load_uniswap_v3(config.quoters.uniswap_v3).await?;
        quoter.load_erc4626(config.quoters.erc4626).await?;

        Ok(quoter)
    }

    fn load_fixed(&mut self, quoters: Vec<FixedQuoter>) {
        for quoter in quoters {
            self.push_quoter(quoter.strip());
        }
    }

    async fn load_uniswap_v2(&mut self, selectors: Vec<UniswapV2Selector>) -> Result<(), JsError> {
        for selector in selectors {
            let quoter = UniswapV2Quoter::from_selector(&self.provider, selector)
                .await
                .map_err(into_js_error)?;
            self.push_quoter(quoter.strip());
        }
        Ok(())
    }

    async fn load_uniswap_v3(&mut self, selectors: Vec<UniswapV3Selector>) -> Result<(), JsError> {
        for selector in selectors {
            let quoter = UniswapV3Quoter::from_selector(&self.provider, selector)
                .await
                .map_err(into_js_error)?;
            self.push_quoter(quoter.strip());
        }
        Ok(())
    }

    async fn load_erc4626(
        &mut self,
        vault_addresses: Vec<alloy::primitives::Address>,
    ) -> Result<(), JsError> {
        for vault_address in vault_addresses {
            let quoter = ERC4626Quoter::new(vault_address, &self.provider)
                .await
                .map_err(into_js_error)?;
            self.push_quoter(quoter.strip());
        }
        Ok(())
    }
}

#[wasm_bindgen(js_name = createEngine)]
pub async fn create_engine(config: JsCreateEngineConfig) -> Result<Engine, JsError> {
    let config: CreateEngineConfig =
        serde_wasm_bindgen::from_value(config.into()).map_err(|e| JsError::new(&e.to_string()))?;
    Engine::from_config(config).await
}
