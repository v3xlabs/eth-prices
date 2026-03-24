use wasm_bindgen::prelude::*;

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

export interface CreateEngineConfig {
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

    #[wasm_bindgen(typescript_type = "CreateEngineConfig")]
    pub type JsCreateEngineConfig;

    #[wasm_bindgen(typescript_type = "QuoteRequest")]
    pub type JsQuoteRequest;

    #[wasm_bindgen(typescript_type = "RouteView")]
    pub type JsRouteView;
}
