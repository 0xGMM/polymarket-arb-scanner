use crate::clob::constants::{
    MARKET_CHANNEL, PING_INTERVAL, RECONNECT_DELAY, USER_CHANNEL, WSS_URL,
};
use crate::clob::internal_event::InternalEvent;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::sleep;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async, tungstenite::protocol::Message,
};
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct ClobWsClient {
    channel_type: String,
    data: Vec<String>,
    auth: Option<serde_json::Value>,
    wss_event_transmitter: UnboundedSender<InternalEvent>,
}

impl ClobWsClient {
    pub fn new(
        channel_type: String,
        data: Vec<String>,
        auth: Option<serde_json::Value>,
        wss_event_tx: UnboundedSender<InternalEvent>,
    ) -> Self {
        Self {
            channel_type,
            data,
            auth,
            wss_event_transmitter: wss_event_tx,
        }
    }

    async fn on_open(
        &self,
        ws_sink: Arc<
            Mutex<
                futures_util::stream::SplitSink<
                    WebSocketStream<MaybeTlsStream<TcpStream>>,
                    Message,
                >,
            >,
        >,
    ) -> tokio::task::JoinHandle<()> {
        let subscribe_msg = if self.channel_type == MARKET_CHANNEL {
            json!({
                "assets_ids": self.data,
                "type": MARKET_CHANNEL
            })
        } else if self.channel_type == USER_CHANNEL
            && let Some(auth) = &self.auth
        {
            json!({
                "markets": self.data,
                "type": USER_CHANNEL,
                "auth": auth
            })
        } else {
            error!("Invalid config for user channel");
            let _ = self.wss_event_transmitter.send(InternalEvent::ClobWsError(
                "Invalid user channel config".to_string(),
            ));
            return tokio::spawn(async {}); // return an empty handle
        };

        {
            let mut sink = ws_sink.lock().await;
            if let Err(e) = sink.send(Message::Text(subscribe_msg.to_string())).await {
                error!("Failed to subscribe: {}", e);
                return tokio::spawn(async {}); // return an empty handle on error
            }
        }

        info!("Subscribed to {} channel", self.channel_type);

        // Keep-alive ping task; return its handle so the caller can abort it.
        let sink_clone = Arc::clone(&ws_sink);
        tokio::spawn(async move {
            loop {
                {
                    let mut sink = sink_clone.lock().await;
                    if sink.send(Message::Ping(vec![])).await.is_err() {
                        break;
                    }
                }
                sleep(PING_INTERVAL).await;
            }
        })
    }

    pub fn on_message(&self, message: Message) {
        match message {
            Message::Text(text) => {
                info!("Message received [{}]: {}", self.channel_type, text);
                let _ = self
                    .wss_event_transmitter
                    .send(InternalEvent::ClobWsRaw(text));
            }
            Message::Close(_) => {
                // At least surface clean close frames.
                info!("Server closed [{}]", self.channel_type);
            }
            _ => {} // ignore Pong, Ping, Binary
        }
    }

    pub fn on_error(&self, err: tokio_tungstenite::tungstenite::Error) {
        error!("WebSocket error: {}", err);
        let _ = self
            .wss_event_transmitter
            .send(InternalEvent::ClobWsError(err.to_string()));
    }

    pub fn on_close(&self) {
        warn!(
            "Connection closed [{}] - reconnecting...",
            self.channel_type
        );
    }

    pub async fn run(&self) {
        let full_url = format!("{}/{}", WSS_URL, self.channel_type);

        loop {
            match connect_async(&full_url).await {
                Ok((ws_stream, _response)) => {
                    info!("Connected [{}]", self.channel_type);

                    let (ws_sink_raw, mut ws_stream) = ws_stream.split();
                    let ws_sink = Arc::new(Mutex::new(ws_sink_raw));

                    let ping_handle = self.on_open(Arc::clone(&ws_sink)).await;

                    while let Some(msg_result) = ws_stream.next().await {
                        match msg_result {
                            Ok(message) => self.on_message(message),
                            Err(e) => {
                                self.on_error(e);
                                break;
                            }
                        }
                    }

                    ping_handle.abort(); // cleanup
                    self.on_close();
                }
                Err(e) => {
                    warn!(
                        "Connection failed [{}]: {}. Retry in {:?}...",
                        self.channel_type, e, RECONNECT_DELAY
                    );
                    sleep(RECONNECT_DELAY).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, sleep};

    // Connects to the live CLOB WebSocket; requires network access.
    #[tokio::test]
    #[ignore = "requires live CLOB WebSocket"]
    async fn test_market_channel_connection_and_messages() {
        // A single known asset_id (a real, active BTC up/down market).
        let asset_ids = vec![
            "27552120954856463326460606808573275580558185382229292475373324804137540385586"
                .to_string(),
        ];

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        let client = ClobWsClient::new(MARKET_CHANNEL.to_string(), asset_ids, None, tx);

        println!("Starting Polymarket WebSocket test (market channel)");
        println!("You should see the subscription plus orderbook messages within 5-10s");

        // Run the client on a separate task so it does not block the test.
        let client_handle = tokio::spawn(async move {
            client.run().await;
        });

        // Let it run for a while to observe live messages.
        sleep(Duration::from_secs(60)).await;

        // run() loops forever, so aborting the task is the clean way to stop it.
        client_handle.abort();
        println!("Test finished - it works if you saw orderbook updates");
    }
}
