use alloy::providers::DynProvider;
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;

pub async fn get_test_provider() -> DynProvider {
    let rpc_url =
        std::env::var("RPC_URL").expect("RPC_URL environment variable must be set to run tests");

    let provider = ProviderBuilder::new().connect(&rpc_url).await.unwrap();

    provider.erased()
}
