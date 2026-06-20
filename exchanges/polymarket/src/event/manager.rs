use crate::event::event::{Event, RawEvent};
use crate::market::market::Market;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

#[derive(Debug, Clone)]
pub struct EventManager {
    my_active_events: Arc<RwLock<HashMap<String, Event>>>,
    allowed_slugs: Vec<String>,
}

impl EventManager {
    pub fn new(allowed_slugs: Vec<String>) -> Arc<Self> {
        Arc::new(Self {
            my_active_events: Arc::new(RwLock::new(HashMap::new())),
            allowed_slugs,
        })
    }

    /// Read-only access to the events we are actively tracking.
    pub async fn active_events(&self) -> RwLockReadGuard<'_, HashMap<String, Event>> {
        self.my_active_events.read().await
    }

    fn filter_events_by_slug(
        &self,
        raw_events: HashMap<String, RawEvent>,
    ) -> HashMap<String, RawEvent> {
        raw_events
            .into_iter()
            .filter(|(_, raw)| {
                if let Some(slug) = &raw.series_slug {
                    self.allowed_slugs.contains(slug)
                } else {
                    false
                }
            })
            .collect()
    }

    fn convert_raw_events_to_events(
        &self,
        raw_events: HashMap<String, RawEvent>,
    ) -> HashMap<String, Event> {
        raw_events
            .into_iter()
            .map(|(id, raw_event)| (id, raw_event.to_event()))
            .collect()
    }

    pub async fn init_active_markets_from_events(
        &self,
        event_id_to_markets: HashMap<String, Vec<(String, Market)>>,
    ) -> Result<()> {
        let mut current_events = self.my_active_events.write().await;

        for (event_id, markets) in event_id_to_markets {
            let event = current_events
                .get_mut(&event_id)
                .ok_or_else(|| anyhow!("Event {} not found in EventManager", event_id))?;

            for (_, market) in markets {
                if market.is_active_market() {
                    let _ = event.add_market(market).await;
                }
            }
        }

        Ok(())
    }

    pub async fn init_my_active_events(&self, raw_events: HashMap<String, RawEvent>) -> Result<()> {
        // Keep only the events matching the allowed slugs.
        let filtered = self.filter_events_by_slug(raw_events);

        let events_map = self.convert_raw_events_to_events(filtered);

        for (_id, event) in events_map {
            if event.active.unwrap_or(false) {
                self.add_active_event(event).await;
            }
        }

        Ok(())
    }

    async fn add_active_event(&self, event: Event) {
        let mut events_map = self.my_active_events.write().await;
        events_map.insert(event.id.clone(), event);
    }
}
