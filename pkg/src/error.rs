#[derive(thiserror::Error, Debug)]
pub enum EthPricesError {
    #[error("Failed to parse configuration: {0}")]
    ConfigError(String),

    #[error("Route computation failed: no path found between {0} and {1}")]
    NoRouteFound(String, String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Token not found: {0}")]
    TokenNotFound(String),

    #[error("Invalid token amount: {0}")]
    InvalidTokenAmount(String),

    #[error("RPC/Contract error: {0}")]
    ContractError(#[from] alloy::contract::Error),

    #[error(transparent)]
    AlloyError(#[from] alloy::transports::TransportError),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T, E = EthPricesError> = std::result::Result<T, E>;
