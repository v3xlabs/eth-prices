#[derive(thiserror::Error, Debug)]
pub enum EthPricesError {
    #[error("Failed to parse configuration: {0}")]
    ConfigError(String),

    #[error("Route computation failed: no path found between {0} and {1}")]
    NoRouteFound(String, String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Invalid network: {0}")]
    InvalidNetwork(String),

    #[error("Token not found: {0}")]
    TokenNotFound(String),

    #[error("Invalid token amount: {0}")]
    InvalidTokenAmount(String),

    #[error("RPC/Contract error: {0}")]
    ContractError(#[from] alloy::contract::Error),

    #[error(transparent)]
    AlloyError(#[from] alloy::transports::TransportError),

    #[error("Routing error: Token missing in computed path")]
    MissingTokenInRoute,

    #[error("Routing error: Quoter missing between path tokens")]
    MissingQuoterInRoute,

    #[error("Routing error: Path length mismatch (expected {expected}, got {actual})")]
    PathLengthMismatch { expected: usize, actual: usize },

    #[error("Parsing error: Invalid fiat symbol")]
    InvalidFiatSymbol,

    #[error("Parsing error: Invalid address - {0}")]
    InvalidAddress(String),

    #[error("ERC4626 vault token must have an on-chain address")]
    MissingVaultAddress,

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("ECB CSV error: {0}")]
    EcbCsvError(String),

    #[error("ECB rate unavailable for fiat:{0}")]
    EcbRateUnavailable(String),
}

pub type Result<T, E = EthPricesError> = std::result::Result<T, E>;
