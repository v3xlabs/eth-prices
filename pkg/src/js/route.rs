use wasm_bindgen::prelude::*;

use crate::router::Route as RouterRoute;

use super::{bindings::JsRouteView, convert::into_js_error, types::RouteView};

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
    pub fn to_json(&self) -> Result<JsRouteView, JsError> {
        serde_wasm_bindgen::to_value(&RouteView::from(&self.inner))
            .map(Into::into)
            .map_err(into_js_error)
    }
}

impl From<RouterRoute> for Route {
    fn from(inner: RouterRoute) -> Self {
        Self { inner }
    }
}
