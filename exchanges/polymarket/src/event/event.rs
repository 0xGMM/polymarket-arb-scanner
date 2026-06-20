use crate::market::market::Market;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize, Clone)]
pub struct RawEvent {
    pub id: String,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub ticker: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,
    pub markets: Option<Vec<Market>>,

    #[serde(rename = "startDate")]
    pub start_date: Option<String>,

    #[serde(rename = "creationDate")]
    pub creation_date: Option<String>,

    #[serde(rename = "endDate")]
    pub end_date: Option<String>,

    #[serde(rename = "enableOrderBook")]
    pub enable_order_book: Option<bool>,

    #[serde(rename = "seriesSlug")]
    pub series_slug: Option<String>,
}

impl RawEvent {
    pub fn get_slug(&self) -> Result<&str> {
        self.slug
            .as_deref()
            .ok_or_else(|| anyhow!("Event slug is missing"))
    }

    pub fn to_event(&self) -> Event {
        // Convert Vec<Market> into a HashMap keyed by market id.
        let markets_map = self
            .markets
            .as_ref()
            .map(|vec| {
                vec.iter()
                    .map(|market| (market.id.clone(), market.clone()))
                    .collect::<HashMap<String, Market>>()
            })
            .unwrap_or_default();

        Event {
            id: self.id.clone(),
            slug: self.slug.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            ticker: self.ticker.clone(),
            active: self.active,
            closed: self.closed,
            markets: Arc::new(RwLock::new(markets_map)),
            start_date: self.start_date.clone(),
            creation_date: self.creation_date.clone(),
            end_date: self.end_date.clone(),
            enable_order_book: self.enable_order_book,
            series_slug: self.series_slug.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub id: String,
    pub markets: Arc<RwLock<HashMap<String, Market>>>,
    pub slug: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub ticker: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,
    pub start_date: Option<String>,
    pub creation_date: Option<String>,
    pub end_date: Option<String>,
    pub enable_order_book: Option<bool>,
    pub series_slug: Option<String>,
}

impl Event {
    pub fn get_slug(&self) -> Result<&str> {
        self.slug
            .as_deref()
            .ok_or_else(|| anyhow!("Event slug is missing"))
    }

    pub fn is_active_event(&self) -> bool {
        let is_open = self.active.unwrap_or(false) && !self.closed.unwrap_or(false);
        let enable_ob = self.enable_order_book.unwrap_or(false);

        if let Some(end_date_str) = &self.end_date
            && let Ok(end_dt) = DateTime::parse_from_rfc3339(end_date_str)
            && end_dt <= Utc::now()
        {
            return false;
        }

        is_open && enable_ob
    }

    pub async fn add_market(&self, market: Market) -> Result<()> {
        let mut markets_map = self.markets.write().await;
        markets_map.insert(market.id.clone(), market);
        Ok(())
    }

    pub async fn remove_market(&self, market_id: &str) -> Result<()> {
        let mut markets_map = self.markets.write().await;
        markets_map.remove(market_id);

        Ok(())
    }

    pub async fn get_market_ids(&self) -> Result<Vec<String>> {
        let map = self.markets.read().await;
        Ok(map.keys().cloned().collect())
    }

    pub async fn read_markets(&self) -> Result<HashMap<String, Market>> {
        let map = self.markets.read().await;
        Ok(map.clone())
    }
}
