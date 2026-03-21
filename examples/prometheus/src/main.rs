use std::{
    collections::{HashMap, HashSet},
    io::Error,
    sync::Arc,
};

use alloy::{
    primitives::address,
    providers::{DynProvider, Provider, ProviderBuilder},
};
use eth_prices::{
    config::Config,
    quoter::{
        Quoter, QuoterInstance, RateDirection,
        uniswap_v2::{UniswapV2Quoter, UniswapV2Selector},
    },
    router::{QuoterGraph, Route},
    token::{Token, TokenIdentifier},
};
use poem::{
    EndpointExt, IntoResponse, Route as PoemRoute, Server, get, handler,
    listener::TcpListener,
    web::{Data, Path},
};
use prometheus_client::{encoding::{EncodeLabelSet, text::encode}, metrics::{family::Family, gauge::Gauge}, registry::Registry};
use tracing_subscriber::fmt;

pub struct ChainState {
    provider: DynProvider,
    router: QuoterGraph,
    all_tokens: HashSet<TokenIdentifier>,
    token_out: TokenIdentifier,
    routes: Vec<Route>,
    quoters: Vec<QuoterInstance>,
}

pub struct Metrics {
    registry: Registry,
    token_price_in_usd: Family<Labels, Gauge>,
}

pub struct AppState {
    config: Config,
    chains: HashMap<String, ChainState>,
    metrics: Metrics,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
struct Labels {
  // Use your own enum types to represent label values.
  chain: String,
  // Or just a plain string.
  token: String,
}

pub async fn setup() -> AppState {
    let config = Config::load("config.toml").await;
    let mut chains = HashMap::new();

    for (chain_slug, chain_config) in &config.chains {
        let url = chain_config.rpc_url.clone();
        let provider = ProviderBuilder::new().connect(&url).await.unwrap().erased();
        for token_config in &chain_config.tokens {
            let token_address = token_config.address.clone();
            println!("token: {:?}", token_address);
        }
        let block = provider.get_block_number().await.unwrap();
        let precision = 10;
        let quoters = chain_config.quoters.all(&provider).await;
        let mut router = QuoterGraph::default();

        for quoter in chain_config.quoters.all(&provider).await {
            router.add_quoter(&quoter);
        }

        let mut all_tokens = HashSet::new();
        for quoter in &quoters {
            let (token_in, token_out) = quoter.get_tokens();
            all_tokens.insert(token_in);
            all_tokens.insert(token_out);
        }

        let token_out = TokenIdentifier::ERC20 {
            address: address!("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"),
        };
        let token_b = Token::new(token_out.clone(), &provider).await.unwrap();
        let mut routes = Vec::new();

        for token in &all_tokens {
            if token == &token_out {
                continue;
            }

            let route = Route::compute(&router, &quoters, &token, &token_out)
                .expect("Failed to compute route");
            println!("route: {:?}", route);
            routes.push(route);
        }

        for route in &routes {
            let token_input = &route.input_token;
            let token_a = Token::new(token_input.clone(), &provider)
                .await
                .unwrap();
            let token_input = token_a.nominal_amount().await;

            let token_output = route.quote(&quoters, block, token_input).await.unwrap();
            println!(
                "token_output: 1 {} = {:?}",
                token_a.symbol,
                token_b.format_amount(token_output, precision).await
            );
        }

        chains.insert(
            chain_slug.clone(),
            ChainState {
                provider,
                router,
                all_tokens,
                token_out,
                routes,
                quoters,
            },
        );
    }

    let mut registry = <Registry>::default();

    let token_price_in_usd = Family::<Labels, Gauge>::default();
    registry.register("token_price_usd", "Token price in USD", token_price_in_usd.clone());

    AppState {
        config,
        chains,
        metrics: Metrics {
            registry,
            token_price_in_usd,
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

        for route in &chain.routes {
            let token_input = &route.input_token;
            let token_input = Token::new(token_input.clone(), &chain.provider).await.unwrap();
            let amount_in = token_input.nominal_amount().await;
            let token_output = route.quote(&chain.quoters, block, amount_in).await.unwrap();
            
            let rate: i64 = token_output.to_string().parse().unwrap();

            state.metrics.token_price_in_usd.get_or_create(
                &Labels {
                    chain: chain_slug.clone(),
                    token: token_input.symbol.clone(),
                }
            ).set(rate);
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
