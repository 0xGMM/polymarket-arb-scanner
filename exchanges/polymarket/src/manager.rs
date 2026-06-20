use crate::binary_market::snapshot::BinaryMarketSnapshot;
use crate::clob::constants::MARKET_CHANNEL;
use crate::clob::internal_event::InternalEvent;
use crate::clob::message::PolymarketMessage;
use crate::clob::ws_client::ClobWsClient;
use crate::constants::{
    ALLOWED_SERIE_SLUGS, BINARY_OUTCOME_COUNT, BINARY_OUTCOMES, BTC_15M_SLUG_PATTERNS,
    PROTOCOL_NAME, SLOW_EVENT_THRESHOLD,
};
use crate::event::event::RawEvent;
use crate::event::manager::EventManager;
use crate::orderbook::manager::OrderbookManager;
use crate::{gamma::client::GammaClient, indexer::PolymarketIndexer};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

#[derive(Debug)]
pub struct PolymarketManager {
    clob_ws_client: Arc<ClobWsClient>,
    orderbook_manager: Arc<OrderbookManager>,
    indexer: Arc<PolymarketIndexer>,
    // Taken once when the event listener starts.
    wss_event_recipient: Mutex<Option<UnboundedReceiver<InternalEvent>>>,
    market_update_transmitter: UnboundedSender<String>,
}

impl PolymarketManager {
    pub async fn new(
        wss_event_transmitter: UnboundedSender<InternalEvent>,
        wss_event_recipient: UnboundedReceiver<InternalEvent>,
        market_update_transmitter: UnboundedSender<String>,
    ) -> Result<Arc<Self>> {
        let gamma_client = GammaClient::new();

        let active_raw_events = gamma_client.get_all_active_events().await?;
        info!("Found {} active events", active_raw_events.len());
        let filtered_events = filter_latest_btc_15m_event(&active_raw_events);
        info!(
            "Filtered to {} events after applying earliest slug pattern : {:?}",
            filtered_events.len(),
            filtered_events
        );

        let event_id_to_markets = gamma_client
            .get_active_markets_from_events(&filtered_events)
            .await?;
        info!(
            "Found {} markets across events",
            event_id_to_markets.values().map(|v| v.len()).sum::<usize>()
        );

        // event_manager
        let event_manager =
            EventManager::new(ALLOWED_SERIE_SLUGS.iter().map(|s| s.to_string()).collect());
        event_manager.init_my_active_events(filtered_events).await?;
        event_manager
            .init_active_markets_from_events(event_id_to_markets)
            .await?;

        for event in event_manager.active_events().await.values() {
            info!("Active event: {:?}", event);
        }

        // orderbook_manager
        let orderbook_manager = OrderbookManager::new();

        // indexer
        let indexer = PolymarketIndexer::new(event_manager.clone())?;
        indexer.update().await?;

        info!("PolymarketIndexer initialized : {:?}", indexer);

        // clob_ws_client
        let asset_ids = indexer.get_all_clob_token_ids().await?;

        info!("Subscribed to {} CLOB tokens", asset_ids.len());

        // Build the WS client with the event sender.
        let clob_ws_client = ClobWsClient::new(
            MARKET_CHANNEL.to_string(),
            asset_ids,
            None,
            wss_event_transmitter,
        );

        let manager = Arc::new(Self {
            clob_ws_client: Arc::new(clob_ws_client),
            orderbook_manager,
            indexer,
            wss_event_recipient: Mutex::new(Some(wss_event_recipient)),
            market_update_transmitter,
        });

        info!("PolymarketManager initialized successfully");

        Ok(manager)
    }

    pub fn start(self: Arc<Self>) -> Vec<JoinHandle<()>> {
        let mut handles = Vec::new();

        let ws_handle = self.start_websocket();
        handles.push(ws_handle);

        let listener_handle = tokio::spawn({
            let manager = Arc::clone(&self);
            async move {
                let Some(wss_event_rx) = manager.wss_event_recipient.lock().await.take() else {
                    error!("wss_event_recipient already consumed; event listener not started");
                    return;
                };

                manager.start_listening_events(wss_event_rx).await;
            }
        });
        handles.push(listener_handle);

        handles
    }

    pub async fn start_listening_events(
        self: Arc<Self>,
        mut wss_event_rx: UnboundedReceiver<InternalEvent>,
    ) {
        info!("[MANAGER] Event listener started");

        while let Some(event) = wss_event_rx.recv().await {
            let manager = self.clone();

            // Process each event on its own task so consumption is never blocked.
            tokio::spawn(async move {
                let start = std::time::Instant::now();
                manager.handle_system_event(event).await;
                let elapsed = start.elapsed();

                if elapsed > SLOW_EVENT_THRESHOLD {
                    warn!("[MANAGER] Slow event processing: {:?}", elapsed);
                }
            });
        }

        warn!("[MANAGER] Event listener stopped");
    }

    pub fn start_websocket(&self) -> JoinHandle<()> {
        let ws_client = self.clob_ws_client.clone();

        tokio::spawn(async move {
            ws_client.run().await;
        })
    }

    async fn handle_system_event(&self, event: InternalEvent) {
        match event {
            InternalEvent::ClobWsRaw(raw) => {
                // Parse directly into the typed message enum.
                match serde_json::from_str::<PolymarketMessage>(&raw) {
                    Ok(PolymarketMessage::SnapshotBatch(snapshots)) => {
                        info!("[MANAGER] Snapshots received, snapshots {:?}", snapshots);

                        for snapshot in snapshots {
                            if let Err(e) = self
                                .orderbook_manager
                                .handle_snapshot_message(snapshot)
                                .await
                            {
                                warn!("[MANAGER] Failed to apply snapshot: {}", e);
                            }
                        }
                    }
                    Ok(PolymarketMessage::PriceChange(update)) => {
                        info!("[MANAGER] PriceChange update {:?}", update);
                        let token_id = update.price_changes.first().map(|c| c.asset_id.clone());

                        // 1. Update the orderbook.
                        if let Err(e) = self
                            .orderbook_manager
                            .handle_price_change_message(update)
                            .await
                        {
                            warn!("[MANAGER] Failed to apply price change: {}", e);
                        }

                        // 2. Notify the scout, if the update carried a token we can map.
                        if let Some(token_id) = token_id
                            && let Ok(Some(market_id)) =
                                self.indexer.get_market_id_from_token(&token_id).await
                        {
                            // market_id is the "0x..." condition_id.
                            info!("[MANAGER] Notifying scout for market_id {}", market_id);

                            self.notify_orderbook_change(market_id).await;
                        }
                    }
                    Err(e) => {
                        error!("[MANAGER] Parse error: {}", e);
                    }
                }
            }
            InternalEvent::ClobWsError(err) => {
                error!("[MANAGER] WS Error: {}", err);
            }
            _ => {}
        }
    }

    pub async fn generate_binary_market_snapshots(
        &self,
    ) -> Result<HashMap<String, BinaryMarketSnapshot>> {
        let mut snapshots = HashMap::new();

        // 1. Full orderbook snapshot (lock-free read, avoids the deadlock path).
        let all_orderbooks = self.orderbook_manager.get_snapshot_parallel().await;

        // 2. All active markets and their token IDs in a single pass.
        let active_market_ids = self.indexer.get_active_market_ids().await?;

        for market_id in active_market_ids {
            // Fetch token IDs (YES/NO).
            let token_ids = match self
                .indexer
                .get_clob_token_ids_for_market(&market_id)
                .await?
            {
                Some(ids) if ids.len() == BINARY_OUTCOME_COUNT => ids,
                _ => continue, // not binary, or missing token IDs
            };

            let (yes_token_id, no_token_id) = (&token_ids[0], &token_ids[1]);

            // Read the orderbooks from the snapshot (no await, no lock).
            let yes_ob = match all_orderbooks.get(yes_token_id) {
                Some(ob) if !ob.is_empty() => ob,
                _ => continue,
            };

            let no_ob = match all_orderbooks.get(no_token_id) {
                Some(ob) if !ob.is_empty() => ob,
                _ => continue,
            };

            let asset_ids = [yes_token_id.to_string(), no_token_id.to_string()];
            // TODO: derive outcomes from market metadata instead of hardcoding.
            let outcomes = BINARY_OUTCOMES.map(|s| s.to_string());
            let obs = [yes_ob.clone(), no_ob.clone()];

            let snapshot = BinaryMarketSnapshot::new(
                market_id.clone(),
                PROTOCOL_NAME.to_string(),
                asset_ids,
                outcomes,
                obs,
            );

            snapshots.insert(market_id, snapshot);
        }

        Ok(snapshots)
    }

    pub async fn generate_single_binary_market_snapshot(
        &self,
        market_id: &str,
    ) -> Result<BinaryMarketSnapshot> {
        // 1. Fetch token IDs (YES/NO).
        let token_ids = match self
            .indexer
            .get_clob_token_ids_for_market(market_id)
            .await?
        {
            Some(ids) if ids.len() == BINARY_OUTCOME_COUNT => ids,
            _ => anyhow::bail!("no token id in market"), // not binary, or missing token IDs
        };

        let (yes_token_id, no_token_id) = (&token_ids[0], &token_ids[1]);

        // 2. Fetch only the two orderbooks we need.
        let orderbooks = self
            .orderbook_manager
            .get_single_market_snapshot(&[yes_token_id.to_string(), no_token_id.to_string()])
            .await?;

        // 3. Make sure both orderbooks exist and have orders.
        let yes_ob = match orderbooks.get(yes_token_id) {
            Some(ob) if !ob.is_empty() => ob,
            _ => anyhow::bail!("no yes ob"),
        };

        let no_ob = match orderbooks.get(no_token_id) {
            Some(ob) if !ob.is_empty() => ob,
            _ => anyhow::bail!("no no ob"),
        };

        // 4. Build the snapshot.
        let asset_ids = [yes_token_id.to_string(), no_token_id.to_string()];
        let outcomes = BINARY_OUTCOMES.map(|s| s.to_string());
        let obs = [yes_ob.clone(), no_ob.clone()];

        let snapshot = BinaryMarketSnapshot::new(
            market_id.to_string(),
            PROTOCOL_NAME.to_string(),
            asset_ids,
            outcomes,
            obs,
        );

        Ok(snapshot)
    }

    pub async fn notify_orderbook_change(&self, market_id: String) {
        let _ = self.market_update_transmitter.send(market_id);
    }
}

fn filter_latest_btc_15m_event(events: &HashMap<String, RawEvent>) -> HashMap<String, RawEvent> {
    let mut latest: Option<(String, RawEvent, i64)> = None;

    for (event_id, event) in events {
        if let Some(slug) = &event.slug {
            let slug_lower = slug.to_lowercase();

            // Match a BTC 15m event slug (with or without dashes).
            if BTC_15M_SLUG_PATTERNS
                .iter()
                .any(|pattern| slug_lower.contains(pattern))
            {
                let parts: Vec<&str> = slug.split('-').collect();
                let timestamp = parts
                    .last()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(i64::MAX);

                match &latest {
                    None => latest = Some((event_id.clone(), event.clone(), timestamp)),
                    Some((_, _, existing_ts)) if timestamp < *existing_ts => {
                        latest = Some((event_id.clone(), event.clone(), timestamp));
                    }
                    _ => {}
                }
            }
        }
    }

    latest
        .map(|(event_id, event, _)| {
            let mut map = HashMap::new();
            map.insert(event_id, event);
            map
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_event(id: &str, slug: &str) -> RawEvent {
        RawEvent {
            id: id.to_string(),
            slug: Some(slug.to_string()),
            title: None,
            description: None,
            ticker: None,
            active: Some(true),
            closed: Some(false),
            markets: None,
            start_date: None,
            creation_date: None,
            end_date: None,
            enable_order_book: Some(true),
            series_slug: None,
        }
    }

    #[test]
    fn picks_the_btc_15m_event_with_the_earliest_timestamp() {
        let mut events = HashMap::new();
        events.insert("a".into(), raw_event("a", "btc-updown-15m-1000"));
        events.insert("b".into(), raw_event("b", "btc-updown-15m-500"));
        events.insert("c".into(), raw_event("c", "btc-updown-15m-1500"));

        let filtered = filter_latest_btc_15m_event(&events);
        assert_eq!(filtered.len(), 1);
        assert!(filtered.contains_key("b"));
    }

    #[test]
    fn ignores_non_btc_15m_events() {
        let mut events = HashMap::new();
        events.insert("x".into(), raw_event("x", "eth-updown-hourly-100"));
        events.insert("y".into(), raw_event("y", "some-other-market"));

        assert!(filter_latest_btc_15m_event(&events).is_empty());
    }

    #[test]
    fn matches_both_slug_spellings() {
        let mut events = HashMap::new();
        events.insert("a".into(), raw_event("a", "btc-up-or-down-15m-300"));
        events.insert("b".into(), raw_event("b", "btc-updown-15m-900"));

        let filtered = filter_latest_btc_15m_event(&events);
        assert_eq!(filtered.len(), 1);
        assert!(filtered.contains_key("a"));
    }
}
