use std::time::Duration;

pub const HOST: &str = "https://clob.polymarket.com";
pub const DEFAULT_MESSAGE: &str = "This message attests that I control the given wallet";
pub const SIGNATURE_TYPE_BROWSER_WALLET: u8 = 2;
pub const WSS_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws";
pub const MARKET_CHANNEL: &str = "market";
pub const USER_CHANNEL: &str = "user";

/// Order side tags used in CLOB price-change messages.
pub const SIDE_BUY: &str = "BUY";
pub const SIDE_SELL: &str = "SELL";

/// Interval between keep-alive pings sent on the CLOB WebSocket.
pub const PING_INTERVAL: Duration = Duration::from_secs(10);

/// Delay before reconnecting after the CLOB WebSocket connection drops.
pub const RECONNECT_DELAY: Duration = Duration::from_secs(3);
