#[derive(Debug, Clone)]
pub enum InternalEvent {
    ClobWsRaw(String), // always the raw message
    ClobWsError(String),
    MarketUpdate(String), // market_id
}
