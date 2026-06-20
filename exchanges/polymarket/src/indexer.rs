use crate::event::manager::EventManager;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct PolymarketIndexer {
    // event_id -> market_ids
    event_to_market_ids: Arc<RwLock<HashMap<String, Vec<String>>>>,

    // market_id -> CLOB token IDs
    market_to_clob_token_ids: Arc<RwLock<HashMap<String, Vec<String>>>>,

    // clob_token_id -> market_id (reverse lookup for the CLOB WS)
    clob_token_to_market: Arc<RwLock<HashMap<String, String>>>,

    // market_id -> event_id
    market_to_event_id: Arc<RwLock<HashMap<String, String>>>,

    active_market_ids: Arc<RwLock<HashSet<String>>>,
    active_event_ids: Arc<RwLock<HashSet<String>>>,
    event_manager: Arc<EventManager>,
}

impl PolymarketIndexer {
    pub fn new(event_manager: Arc<EventManager>) -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            event_to_market_ids: Arc::new(RwLock::new(HashMap::new())),
            market_to_clob_token_ids: Arc::new(RwLock::new(HashMap::new())),
            clob_token_to_market: Arc::new(RwLock::new(HashMap::new())),
            market_to_event_id: Arc::new(RwLock::new(HashMap::new())),
            active_market_ids: Arc::new(RwLock::new(HashSet::new())),
            active_event_ids: Arc::new(RwLock::new(HashSet::new())),
            event_manager,
        }))
    }

    pub async fn update(&self) -> Result<()> {
        let events = self.event_manager.active_events().await;

        let mut event_to_markets = HashMap::new();
        let mut market_to_tokens = HashMap::new();
        let mut token_to_market = HashMap::new();
        let mut market_to_event = HashMap::new();
        let mut active_markets = HashSet::new();
        let mut active_events = HashSet::new();

        for (event_id, event) in events.iter() {
            let event_active = event.is_active_event();
            if event_active {
                active_events.insert(event_id.clone());
            }

            let markets_guard = event.markets.read().await;
            let market_ids = event.get_market_ids().await?;

            for (market_id, market) in markets_guard.iter() {
                market_to_event.insert(market_id.clone(), event_id.clone());

                // Clone only to run the mutable parsing step.
                let mut market_for_compute = market.clone();
                market_for_compute.compute_clob_token_ids_formatted();

                // Extract the token IDs without moving out of the market.
                if let Some(tokens) = &market_for_compute.clob_token_ids_formatted {
                    let tokens_vec = tokens.clone();
                    market_to_tokens.insert(market_id.clone(), tokens_vec.clone());

                    for token_id in tokens_vec {
                        token_to_market.insert(token_id, market_id.clone());
                    }
                }

                // is_active_market() does not depend on parsing, so use the original market.
                if market.is_active_market() && event_active {
                    active_markets.insert(market_id.clone());
                }
            }

            if !market_ids.is_empty() {
                event_to_markets.insert(event_id.clone(), market_ids);
            }
        }

        // Swap in the freshly built indexes.
        *self.event_to_market_ids.write().await = event_to_markets;
        *self.market_to_clob_token_ids.write().await = market_to_tokens;
        *self.clob_token_to_market.write().await = token_to_market;
        *self.market_to_event_id.write().await = market_to_event;
        *self.active_market_ids.write().await = active_markets;
        *self.active_event_ids.write().await = active_events;

        Ok(())
    }

    pub async fn get_all_clob_token_ids(&self) -> Result<Vec<String>> {
        let map = self.market_to_clob_token_ids.read().await;
        Ok(map.values().flat_map(|v| v.iter().cloned()).collect())
    }

    pub async fn get_active_market_ids(&self) -> Result<HashSet<String>> {
        let active_market_ids = self.active_market_ids.read().await;
        Ok(active_market_ids.clone())
    }

    pub async fn get_active_event_ids(&self) -> Result<HashSet<String>> {
        let active = self.active_event_ids.read().await;
        Ok(active.clone())
    }

    pub async fn get_event_id_for_market(&self, market_id: &String) -> Result<Option<String>> {
        let map = self.market_to_event_id.read().await;
        Ok(map.get(market_id).cloned())
    }

    pub async fn get_market_ids_for_event(&self, event_id: &String) -> Result<Option<Vec<String>>> {
        let map = self.event_to_market_ids.read().await;
        Ok(map.get(event_id).cloned())
    }

    pub async fn get_clob_token_ids_for_market(
        &self,
        market_id: &str,
    ) -> Result<Option<Vec<String>>> {
        let map = self.market_to_clob_token_ids.read().await;
        Ok(map.get(market_id).cloned())
    }

    pub async fn get_clob_token_ids_to_market(&self) -> Result<HashMap<String, String>> {
        let map = self.clob_token_to_market.read().await;
        Ok(map.clone())
    }

    pub async fn get_all_market_ids_for_event(&self, event_id: &str) -> Result<Vec<String>> {
        let map = self.event_to_market_ids.read().await;
        Ok(map.get(event_id).cloned().unwrap_or_default())
    }

    pub async fn active_market_count(&self) -> usize {
        self.active_market_ids.read().await.len()
    }

    pub async fn active_event_count(&self) -> usize {
        self.active_event_ids.read().await.len()
    }

    pub async fn get_market_id_from_token(&self, token_id: &str) -> Result<Option<String>> {
        let map = self.clob_token_to_market.read().await;
        Ok(map.get(token_id).cloned())
    }
}
