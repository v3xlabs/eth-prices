use wasm_bindgen::prelude::*;

use crate::router::Route as RouterRoute;

use super::{bindings::JsRoutePath, types::route_path};

fn into_js_error(err: impl Into<anyhow::Error>) -> JsError {
    JsError::new(&err.into().to_string())
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct Route {
    pub(crate) inner: RouterRoute,
}

#[wasm_bindgen]
impl Route {
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsRoutePath, JsError> {
        serde_wasm_bindgen::to_value(&route_path(&self.inner))
            .map(Into::into)
            .map_err(into_js_error)
    }

    pub fn path(&self) -> Result<JsRoutePath, JsError> {
        self.to_json()
    }
}

impl From<RouterRoute> for Route {
    fn from(inner: RouterRoute) -> Self {
        Self { inner }
    }
}
