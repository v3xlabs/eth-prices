use alloy::providers::{Provider, ProviderBuilder};

use crate::provider::RpcProvider;

pub async fn get_test_provider() -> RpcProvider {
    let rpc_url =
        std::env::var("RPC_URL").expect("RPC_URL environment variable must be set to run tests");

    let provider = ProviderBuilder::new().connect(&rpc_url).await.unwrap();

    provider.erased()
}
