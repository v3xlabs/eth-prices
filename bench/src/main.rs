use std::{
    collections::HashMap, env, fs, hint::black_box, path::PathBuf, str::FromStr, time::Instant,
};

use alloy::{
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder},
};
use eth_prices::{asset::AssetIdentifier, network::NetworkInstant, router::AutoRouter};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    network_id: u64,
    v2_factory: String,
    v3_factory: String,
    v3_fees: Vec<u32>,
    assets: HashMap<String, AssetConfig>,
    runs: Vec<RunConfig>,
}

#[derive(Debug, Deserialize)]
struct AssetConfig {
    address: String,
    decimals: u8,
}

#[derive(Debug, Deserialize)]
struct RunConfig {
    name: String,
    tokens: Vec<String>,
    quotes: Vec<String>,
    protocols: ProtocolConfig,
}

#[derive(Debug, Deserialize)]
struct ProtocolConfig {
    v2: bool,
    v3: bool,
    erc4626: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvalOutput {
    implementation: &'static str,
    block_number: u64,
    discovery_block_pinned: bool,
    iterations: u32,
    runs: Vec<RunOutput>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunOutput {
    name: String,
    discovery_ms: f64,
    discovered_quoters: usize,
    quotes: Vec<QuoteOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuoteOutput {
    asset: String,
    output_asset: &'static str,
    input_amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    route_compute_ns: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quote_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hops: Option<usize>,
    sources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn elapsed_ms(started_at: Instant) -> f64 {
    started_at.elapsed().as_secs_f64() * 1_000.0
}

fn nominal_amount(decimals: u8) -> Result<U256, String> {
    U256::from_str(&format!("1{}", "0".repeat(usize::from(decimals))))
        .map_err(|error| error.to_string())
}

fn asset_identifier(asset: &AssetConfig) -> Result<AssetIdentifier, String> {
    AssetIdentifier::try_from(asset.address.as_str()).map_err(|error| error.to_string())
}

fn manifest_path() -> PathBuf {
    env::var_os("EVAL_CASES")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cases.json"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest: Manifest = serde_json::from_str(&fs::read_to_string(manifest_path())?)?;
    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "https://ethereum.reth.rs/rpc".to_owned());
    let iterations = env::var("EVAL_ITERATIONS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1_000)
        .max(1);
    let provider = ProviderBuilder::new()
        .connect(rpc_url.as_str())
        .await?
        .erased();
    let block_number = match env::var("EVAL_BLOCK") {
        Ok(value) => value.parse::<u64>()?,
        Err(_) => provider.get_block_number().await?,
    };
    let network = NetworkInstant::default().with_evm_block(
        manifest.network_id.into(),
        block_number,
        provider.clone(),
    );
    let v2_factory = Address::from_str(&manifest.v2_factory)?;
    let v3_factory = Address::from_str(&manifest.v3_factory)?;
    let mut run_outputs = Vec::with_capacity(manifest.runs.len());

    for run in manifest.runs {
        let tokens = run
            .tokens
            .iter()
            .map(|name| {
                manifest
                    .assets
                    .get(name)
                    .ok_or_else(|| format!("unknown asset {name}"))
                    .and_then(asset_identifier)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let started_at = Instant::now();
        let router_result = AutoRouter::new(provider.clone(), tokens)
            .with_network_id(manifest.network_id.into())
            .with_uniswap_v2_factory(v2_factory)
            .with_uniswap_v3_factory(v3_factory)
            .with_uniswap_v3_fees(manifest.v3_fees.clone())
            .discover_uniswap_v2(run.protocols.v2)
            .discover_uniswap_v3(run.protocols.v3)
            .discover_erc4626(run.protocols.erc4626)
            .build()
            .await;
        let discovery_ms = elapsed_ms(started_at);

        let router = match router_result {
            Ok(router) => router,
            Err(error) => {
                run_outputs.push(RunOutput {
                    name: run.name,
                    discovery_ms,
                    discovered_quoters: 0,
                    quotes: Vec::new(),
                    error: Some(error.to_string()),
                });
                continue;
            }
        };
        let output_asset = manifest.assets.get("usdc").ok_or("missing usdc asset")?;
        let output_identifier = asset_identifier(output_asset)?;
        let mut quote_outputs = Vec::with_capacity(run.quotes.len());

        for asset_name in run.quotes {
            let asset = manifest
                .assets
                .get(&asset_name)
                .ok_or_else(|| format!("unknown quote asset {asset_name}"))?;
            let input_identifier = asset_identifier(asset)?;
            let amount_in = nominal_amount(asset.decimals)?;
            let route = match router.compute(&input_identifier, &output_identifier) {
                Ok(route) => route,
                Err(error) => {
                    quote_outputs.push(QuoteOutput {
                        asset: asset_name,
                        output_asset: "usdc",
                        input_amount: amount_in.to_string(),
                        output_amount: None,
                        route_compute_ns: None,
                        quote_ms: None,
                        hops: None,
                        sources: Vec::new(),
                        error: Some(error.to_string()),
                    });
                    continue;
                }
            };
            let route_started_at = Instant::now();
            for _ in 0..iterations {
                black_box(
                    router.compute(black_box(&input_identifier), black_box(&output_identifier))?,
                );
            }
            let route_compute_ns =
                route_started_at.elapsed().as_secs_f64() * 1_000_000_000.0 / f64::from(iterations);
            let sources = route
                .path
                .iter()
                .map(|step| step.quoter.to_string())
                .collect();
            let quote_started_at = Instant::now();
            let quote_result = route.quote(&network, amount_in).await;
            let quote_ms = elapsed_ms(quote_started_at);

            match quote_result {
                Ok(amount_out) => quote_outputs.push(QuoteOutput {
                    asset: asset_name,
                    output_asset: "usdc",
                    input_amount: amount_in.to_string(),
                    output_amount: Some(amount_out.to_string()),
                    route_compute_ns: Some(route_compute_ns),
                    quote_ms: Some(quote_ms),
                    hops: Some(route.path.len()),
                    sources,
                    error: None,
                }),
                Err(error) => quote_outputs.push(QuoteOutput {
                    asset: asset_name,
                    output_asset: "usdc",
                    input_amount: amount_in.to_string(),
                    output_amount: None,
                    route_compute_ns: Some(route_compute_ns),
                    quote_ms: Some(quote_ms),
                    hops: Some(route.path.len()),
                    sources,
                    error: Some(error.to_string()),
                }),
            }
        }

        run_outputs.push(RunOutput {
            name: run.name,
            discovery_ms,
            discovered_quoters: router.quoters.len(),
            quotes: quote_outputs,
            error: None,
        });
    }

    println!(
        "{}",
        serde_json::to_string(&EvalOutput {
            implementation: "rust",
            block_number,
            discovery_block_pinned: false,
            iterations,
            runs: run_outputs,
        })?
    );

    Ok(())
}
