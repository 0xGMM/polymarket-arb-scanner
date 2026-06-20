use alloy::primitives::{Address, U256};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct ClobAuth {
    pub address: Address,
    pub message: String,
    pub timestamp: String,
    pub nonce: U256,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthPayload {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    pub secret: String,
    pub passphrase: String,
}
