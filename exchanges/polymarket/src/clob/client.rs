use crate::clob::auth::AuthPayload;
use alloy::primitives::Address;

#[derive(Debug, Clone)]
pub struct ClobClient {
    pub host: String,
    pub key: String,
    pub chain_id: u64,
    pub signature_type: u8,
    pub funder: Address,
    pub auth: AuthPayload,
}
