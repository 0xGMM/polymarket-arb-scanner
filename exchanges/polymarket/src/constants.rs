use alloy::primitives::{Address, address};
use phf::phf_map;
use std::time::Duration;

pub const PROTOCOL_NAME: &str = "Polymarket";

// MARKET MODEL
/// Number of outcomes in a binary market.
pub const BINARY_OUTCOME_COUNT: usize = 2;
/// Outcome labels for a binary (YES/NO) market.
pub const BINARY_OUTCOMES: [&str; BINARY_OUTCOME_COUNT] = ["YES", "NO"];
/// Payout in USD of one matched YES+NO share pair at resolution.
pub const SHARE_PAYOFF_USD: f64 = 1.0;
/// Slug patterns identifying a BTC 15-minute up/down event.
pub const BTC_15M_SLUG_PATTERNS: [&str; 2] = ["btc-updown-15m", "btc-up-or-down-15m"];

// EVENT PROCESSING
/// Event-processing time above which a slowdown warning is logged.
pub const SLOW_EVENT_THRESHOLD: Duration = Duration::from_millis(10);

// RATE LIMIT
pub const GENERAL_RATE_LIMIT_PER_SEC: u16 = 500;
pub const OK_ENDPOINT_RATE_LIMIT_PER_SEC: u8 = 5;
pub const CLOB_RATE_LIMIT_PER_SEC: u16 = 500;

// CONTRACT MAPPINGS
pub static CHAIN_ID_TO_CTF_EXCHANGE_CONTRACT_ADDRESS: phf::Map<u64, Address> = phf_map! {
    137u64  => address!("4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E"),
};

pub static CHAIN_ID_TO_NEGRISK_CTF_EXCHANGE_CONTRACT_ADDRESS: phf::Map<u64, Address> = phf_map! {
    137u64  => address!("C5d563A36AE78145C45a50134d48A1215220f80A"),
};

pub static CHAIN_ID_TO_SAFE_PROXY_FACTORY_CONTRACT_ADDRESS: phf::Map<u64, Address> = phf_map! {
    137u64  => address!("aacfEEa03eb1561C4e67d661e40682Bd20E3541B"),
};

pub static CHAIN_ID_TO_SAFE_MULTISEND_CONTRACT_ADDRESS: phf::Map<u64, Address> = phf_map! {
    137u64  => address!("A238CBeb142c10Ef7Ad8442C6D1f9E89e07e7761"),
};

pub const POLYGON_CHAIN_ID: u64 = 137;

pub const ALLOWED_SERIE_SLUGS: &[&str] = &["btc-up-or-down-15m"];
