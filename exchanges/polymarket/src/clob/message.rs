use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PolymarketMessage {
    #[serde(rename = "book")]
    SnapshotBatch(Vec<BookSnapshotMessage>),

    #[serde(rename = "price_change")]
    PriceChange(PriceChangeMessage),
}

#[derive(Debug, Deserialize)]
pub struct BookLevel {
    pub price: String,
    pub size: String,
}

#[derive(Debug, Deserialize)]
pub struct BookSnapshotMessage {
    pub asset_id: String,
    pub market: String,
    pub timestamp: String,
    pub hash: String,
    #[serde(default)]
    pub bids: Vec<BookLevel>,
    #[serde(default)]
    pub asks: Vec<BookLevel>,
    pub event_type: String, // "book"
}

#[derive(Debug, Deserialize)]
pub struct PriceChangeMessage {
    pub timestamp: String,
    #[serde(default)]
    pub price_changes: Vec<PriceLevelUpdate>,
    pub event_type: String, // "price_change"
}

#[derive(Debug, Deserialize)]
pub struct PriceLevelUpdate {
    pub asset_id: String,
    pub side: String,
    pub price: String,
    pub size: String,
    pub hash: String,
}
