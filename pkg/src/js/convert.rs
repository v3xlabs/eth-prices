use alloy::primitives::{Address, U256};
use anyhow::{Result, anyhow};
use wasm_bindgen::prelude::*;

use crate::token::TokenIdentifier;

pub fn into_js_error(err: impl Into<anyhow::Error>) -> JsError {
    JsError::new(&err.into().to_string())
}

pub fn parse_token_identifier(token: &str) -> Result<TokenIdentifier, JsError> {
    TokenIdentifier::try_from(token.to_string()).map_err(into_js_error)
}

pub fn parse_u256(amount: &str) -> Result<U256, JsError> {
    amount
        .parse::<U256>()
        .map_err(|err| into_js_error(anyhow!(err)))
}

pub fn parse_address(address: &str) -> Result<Address, JsError> {
    address
        .parse::<Address>()
        .map_err(|err| into_js_error(anyhow!(err)))
}
