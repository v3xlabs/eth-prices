/*!
 * JS Compatibility layer using wasm-bindgen.
 */
use serde::{Deserialize, Serialize};
use wasm_bindgen::{convert::TryFromJsValue, prelude::*};
use js_sys::JsString;

#[wasm_bindgen]
#[derive(Debug, PartialEq, Clone)]
pub struct QuoterConfig {
    pub test: JsString,
}

#[wasm_bindgen]
extern "C" {
    pub type QuoterConfig;
}

/// Create a new quoter from a configuration object.
///
/// # Arguments
///
/// * `config` - A configuration object.
///
/// # Returns
///
/// A new quoter instance.
#[wasm_bindgen]
pub fn createQuoter(config: QuoterConfig) -> Result<JsValue, JsError> {
    Ok(JsValue::null())
}
