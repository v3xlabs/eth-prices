use wasm_bindgen::prelude::*;

use super::types::RouteView;
use crate::router::Route as RouterRoute;

#[wasm_bindgen]
#[derive(Clone)]
pub struct Route {
    pub(crate) inner: RouterRoute,
}

#[wasm_bindgen]
impl Route {
    #[wasm_bindgen(js_name = inputToken)]
    pub fn input_token(&self) -> String {
        self.inner.input_token.to_string()
    }

    #[wasm_bindgen(js_name = outputToken)]
    pub fn output_token(&self) -> String {
        self.inner.output_token.to_string()
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<RouteView, JsError> {
        Ok(RouteView::from(&self.inner))
    }
}

impl From<RouterRoute> for Route {
    fn from(inner: RouterRoute) -> Self {
        Self { inner }
    }
}
