//! European Central Bank (ECB) Quoter
//! 
//! The [`EcbRateSource`] struct is used to fetch rates from the European Central Bank (ECB) API.
//! 
//! ```rust,ignore
//! use eth_prices::quoter::ecb::EcbRateSource;
//! 
//! let rates = EcbRateSource::default();
//! let fiat_graph = rates.graph();
//! 
//! let token_in: TokenIdentifier = "fiat:eur".try_into().unwrap();
//! let token_out: TokenIdentifier = "fiat:czk".try_into().unwrap();
//! let network = Network::Fiat(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
//! 
//! let route = fiat_graph.compute(&token_in, &token_out).unwrap();
//! let quote = route.quote(&network, U256::from(1_000_000)).await.unwrap();
//! 
//! println!("quote: {:?}", quote);
//! ```
//! 

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use alloy::primitives::{U256, aliases::U2048};

use crate::{
    Result, error::EthPricesError, network::Network, quoter::{AnyQuoter, Quoter, RateDirection}, router::graph::QuoterGraph, asset::identity::AssetIdentifier
};

const ECB_BASE_URL: &str = "https://data-api.ecb.europa.eu/service/data/EXR/D..EUR.SP00.A";
const RATE_DECIMALS: u8 = 6;
const EUR: &str = "eur";
pub const ECB_CURRENCIES: &[&str] = &[
    "ars", "aud", "bgn", "brl", "cad", "chf", "cny", "cyp", "czk", "dkk", "dzd", "eek", "gbp",
    "grd", "hkd", "hrk", "huf", "idr", "ils", "inr", "isk", "jpy", "krw", "ltl", "lvl", "mad",
    "mtl", "mxn", "myr", "nok", "nzd", "php", "pln", "ron", "rub", "sek", "sgd", "sit", "skk",
    "thb", "try", "twd", "usd", "zar",
];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum EcbCacheKey {
    Date(String),
}

#[derive(Debug, Clone, Default)]
pub struct EcbRateSource {
    cache: Arc<Mutex<HashMap<EcbCacheKey, Arc<HashMap<String, U256>>>>>,
}

impl EcbRateSource {
    pub fn graph(&self) -> QuoterGraph {
        let mut graph = QuoterGraph::default();
        for quoter in self.quoters() {
            graph.add_quoter(AnyQuoter(Arc::new(quoter)));
        }
        graph
    }

    pub fn quoters(&self) -> Vec<EcbQuoter> {
        ECB_CURRENCIES
            .iter()
            .map(|symbol| EcbQuoter {
                quote_symbol: (*symbol).to_string(),
                rates: self.clone(),
            })
            .collect()
    }

    async fn rate_for(
        &self,
        symbol: &str,
        network: &Network,
    ) -> Result<U256> {
        let key = EcbCacheKey::Date(unix_timestamp_to_date(*network.as_fiat().unwrap()));

        if let Some(rates) = self
            .cache
            .lock()
            .expect("ecb cache mutex poisoned")
            .get(&key)
            .cloned()
        {
            return rates
                .get(symbol)
                .copied()
                .ok_or_else(|| EthPricesError::EcbRateUnavailable(symbol.to_string()));
        }

        let rates = Arc::new(fetch_rates(&key).await?);
        self.cache
            .lock()
            .expect("ecb cache mutex poisoned")
            .insert(key, rates.clone());
        rates
            .get(symbol)
            .copied()
            .ok_or_else(|| EthPricesError::EcbRateUnavailable(symbol.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct EcbQuoter {
    quote_symbol: String,
    rates: EcbRateSource,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Quoter for EcbQuoter {
    fn identity(&self) -> String {
        format!("ecb:fiat:{EUR}:fiat:{}", self.quote_symbol)
    }

    fn tokens(&self) -> (AssetIdentifier, AssetIdentifier) {
        (
            AssetIdentifier::Fiat {
                symbol: EUR.to_string(),
            },
            AssetIdentifier::Fiat {
                symbol: self.quote_symbol.clone(),
            },
        )
    }

    async fn rate(
        &self,
        amount_in: U256,
        direction: RateDirection,
        network: &Network,
    ) -> Result<U256> {
        let rate = self
            .rates
            .rate_for(&self.quote_symbol, network)
            .await?;
        let amount_in = U2048::from(amount_in);
        let scale = U2048::from(10).pow(U2048::from(RATE_DECIMALS));
        let quoted = match direction {
            RateDirection::Forward => amount_in * U2048::from(rate) / scale,
            RateDirection::Reverse => amount_in * scale / U2048::from(rate),
        };
        Ok(U256::from(quoted))
    }
}

async fn fetch_rates(key: &EcbCacheKey) -> Result<HashMap<String, U256>> {
    let url = match key {
        EcbCacheKey::Date(date) => {
            format!("{ECB_BASE_URL}?endPeriod={date}&lastNObservations=1&format=csvdata")
        }
    };
    let body = reqwest::get(url).await?.error_for_status()?.text().await?;
    parse_rates(&body)
}

fn parse_rates(body: &str) -> Result<HashMap<String, U256>> {
    let mut reader = csv::Reader::from_reader(body.as_bytes());
    let mut rates = HashMap::new();
    for record in reader.records() {
        let record = record.map_err(|error| EthPricesError::EcbCsvError(error.to_string()))?;
        let symbol = record
            .get(2)
            .ok_or_else(|| EthPricesError::EcbCsvError("missing CURRENCY column".to_string()))?
            .to_ascii_lowercase();
        let value = record
            .get(7)
            .ok_or_else(|| EthPricesError::EcbCsvError("missing OBS_VALUE column".to_string()))?;
        rates.insert(symbol, parse_decimal_rate(value)?);
    }
    Ok(rates)
}

fn parse_decimal_rate(value: &str) -> Result<U256> {
    let (integer, fractional) = value.split_once('.').unwrap_or((value, ""));
    let mut fractional = fractional.to_string();
    fractional.truncate(RATE_DECIMALS as usize);
    while fractional.len() < RATE_DECIMALS as usize {
        fractional.push('0');
    }
    let scaled = format!("{integer}{fractional}")
        .parse::<u128>()
        .map_err(|error| EthPricesError::EcbCsvError(error.to_string()))?;
    Ok(U256::from(scaled))
}

fn unix_timestamp_to_date(timestamp: u64) -> String {
    let days = (timestamp / 86_400) as i64;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days: i64) -> (i64, u64, u64) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };
    (year, month as u64, day as u64)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn parses_ecb_csv_rates() {
        let rates = parse_rates("KEY,FREQ,CURRENCY,CURRENCY_DENOM,EXR_TYPE,EXR_SUFFIX,TIME_PERIOD,OBS_VALUE\nEXR.D.USD.EUR.SP00.A,D,USD,EUR,SP00,A,2026-05-14,1.1702\nEXR.D.CZK.EUR.SP00.A,D,CZK,EUR,SP00,A,2026-05-14,24.302\n").unwrap();
        assert_eq!(rates["usd"], U256::from(1_170_200));
        assert_eq!(rates["czk"], U256::from(24_302_000));
    }

    #[test]
    fn formats_unix_timestamp_as_date() {
        assert_eq!(unix_timestamp_to_date(0), "1970-01-01");
    }

    #[test]
    fn creates_eur_hub_quoters() {
        let rates = EcbRateSource::default();
        let quoters = rates.quoters();
        let usd = quoters
            .iter()
            .find(|quoter| quoter.quote_symbol == "usd")
            .unwrap();
        assert_eq!(
            usd.tokens(),
            (
                AssetIdentifier::Fiat {
                    symbol: "eur".to_string()
                },
                AssetIdentifier::Fiat {
                    symbol: "usd".to_string()
                }
            )
        );
        assert_eq!(usd.identity(), "ecb:fiat:eur:fiat:usd");
    }

    #[tokio::test]
    async fn creates_ecb_graph() {
        let rates = EcbRateSource::default();
        let graph = rates.graph();
        // assert_eq!(graph.quoters().len(), ECB_CURRENCIES.len());

        let token_in = AssetIdentifier::Fiat { symbol: "eur".to_string() };
        let token_out = AssetIdentifier::Fiat { symbol: "czk".to_string() };

        // quote 1 eur to sek
        let route = graph.compute(&token_in, &token_out).unwrap();

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let network = Network::Fiat(now);

        let quote = route.quote(&network, U256::from(1_000_000)).await.unwrap();

        println!("quote: {:?}", quote);
    }
}
