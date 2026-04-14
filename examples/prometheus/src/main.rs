use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use eth_prices::config::Config;
use eth_prices::router::Route;
use eth_prices::router::graph::QuoterGraph;
use eth_prices::token::{Token, TokenIdentifier};
use poem::listener::TcpListener;
use poem::web::Data;
use poem::{EndpointExt, Route as PoemRoute, Server, get, handler};
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;
use std::collections::{HashMap, HashSet};
use std::io::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tracing::info;

pub struct ChainState {
    provider: DynProvider,
    #[allow(dead_code)]
    router: QuoterGraph,
    routes: Vec<Route>,
}

pub struct Metrics {
    registry: Registry,
    token_price_in_usd: Family<TokenLabels, Gauge<f64, AtomicU64>>,
    block_height: Family<Labels, Gauge<u64, AtomicU64>>,
}

pub struct AppState {
    #[allow(dead_code)]
    config: Config,
    chains: HashMap<String, ChainState>,
    metrics: Metrics,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct TokenLabels {
    // Use your own enum types to represent label values.
    chain: String,
    // Or just a plain string.
    token: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct Labels {
    // Use your own enum types to represent label values.
    chain: String,
}

pub async fn setup() -> AppState {
    let config = Config::load("config.toml").await.unwrap();
    let mut chains = HashMap::new();

    for (chain_slug, chain_config) in &config.chains {
        let url = chain_config.rpc_url.clone();
        let provider = ProviderBuilder::new().connect(&url).await.unwrap().erased();
        for token_config in &chain_config.tokens {
            let token_address = token_config.address.clone();
            println!("token: {:?}", token_address);
        }
        let quoters = chain_config.quoters.clone().all(&provider).await.unwrap();
        let router = QuoterGraph::from_iter(quoters);

        let mut all_tokens = HashSet::new();
        for quoter in &router.quoters {
            let (token_in, token_out) = quoter.tokens();
            all_tokens.insert(token_in);
            all_tokens.insert(token_out);
        }

        let token_out = TokenIdentifier::Fiat {
            symbol: "usd".to_string(),
        };
        let mut routes = Vec::new();

        for token in &all_tokens {
            if token == &token_out {
                continue;
            }

            let route = router
                .compute(token, &token_out)
                .expect("Failed to compute route");
            info!("route: {:?}", route);
            routes.push(route);
        }

        chains.insert(
            chain_slug.clone(),
            ChainState {
                provider,
                router,
                routes,
            },
        );
    }

    let mut registry = <Registry>::default();

    let token_price_in_usd = Family::<TokenLabels, Gauge<f64, AtomicU64>>::default();
    registry.register(
        "token_price_usd",
        "Token price in USD",
        token_price_in_usd.clone(),
    );

    let block_height = Family::<Labels, Gauge<u64, AtomicU64>>::default();
    registry.register("block_height", "Block height", block_height.clone());

    AppState {
        config,
        chains,
        metrics: Metrics {
            registry,
            token_price_in_usd,
            block_height,
        },
    }
}

#[handler]
fn index() -> String {
    "hello world!".to_string()
}

#[handler]
async fn metrics(state: Data<&Arc<AppState>>) -> String {
    for (chain_slug, chain) in &state.chains {
        let block = chain.provider.get_block_number().await.unwrap();

        state
            .metrics
            .block_height
            .get_or_create(&Labels {
                chain: chain_slug.clone(),
            })
            .set(block);

        for route in &chain.routes {
            let token_input = &route.input_token;
            let token_input = Token::new(token_input.clone(), &chain.provider)
                .await
                .unwrap();
            let amount_in = token_input.nominal_amount().await;
            let token_output = route.quote(block, amount_in).await.unwrap();

            let rate: i64 = token_output.to_string().parse().unwrap();
            let rate = rate as f64 / 10_f64.powf(6_f64);

            state
                .metrics
                .token_price_in_usd
                .get_or_create(&TokenLabels {
                    chain: chain_slug.clone(),
                    token: token_input.symbol.clone(),
                })
                .set(rate);
        }
    }

    let mut buffer = String::new();
    encode(&mut buffer, &state.metrics.registry).unwrap();

    buffer
}

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let state = setup().await;

    let app = PoemRoute::new()
        .at("/", get(index))
        .at("/metrics", get(metrics))
        .data(Arc::new(state));

    Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await
}
