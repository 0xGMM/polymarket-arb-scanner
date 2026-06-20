//! Client for the Polymarket Gamma REST API (event/market discovery).
//!
//! NOTE: this targets the Gamma API as it existed in late 2025. The upstream
//! endpoints and response shapes have since changed, so these calls may no
//! longer succeed against the current API without updates.

use crate::{
    event::event::{Event, RawEvent},
    gamma::constants::{ERROR_PREVIEW_CHARS, EVENTS_SLUG, GAMMA_URL, LIMIT_EVENTS, MARKETS_SLUG},
    market::market::Market,
    serie::serie::Serie,
};
use anyhow::{Result, anyhow, ensure};
use reqwest::Client;
use std::collections::HashMap;
use tracing::{debug, error, info};

#[derive(Debug, Clone)]
pub struct GammaClient {
    http: Client,
}

impl Default for GammaClient {
    fn default() -> Self {
        Self::new()
    }
}

impl GammaClient {
    /// Create a new Gamma client
    pub fn new() -> Self {
        Self {
            http: Client::new(),
        }
    }

    // --------------------------------------
    // MARKETS
    // --------------------------------------

    /// GET /markets — Retrieve all markets
    pub async fn get_markets(&self) -> Result<Vec<Market>> {
        let url = format!("{}/markets", GAMMA_URL);

        let res = self.http.get(&url).send().await?;

        ensure!(
            res.status().is_success(),
            "Gamma API error: {}",
            res.status()
        );
        let markets = res.json::<Vec<Market>>().await?;

        Ok(markets)
    }

    /// GET /markets/{id}
    pub async fn get_market_by_id(&self, id: &str) -> Result<Market> {
        let url = format!("{}/markets?tag_id={}", GAMMA_URL, id);

        debug!("GET {}", url);
        let res = self.http.get(&url).send().await?;

        ensure!(
            res.status().is_success(),
            "Gamma API error: {}",
            res.status()
        );
        let text = res.text().await?;

        match serde_json::from_str::<Market>(&text) {
            Ok(market) => Ok(market),
            Err(e) => {
                error!("Serde error: {:?}", e);
                Err(e.into())
            }
        }
    }

    /// GET /markets/slug/{slug}
    pub async fn get_market_by_slug(&self, slug: &str) -> Result<Market> {
        let url = format!("{}{}{}", GAMMA_URL, MARKETS_SLUG, slug);
        let res = self.http.get(&url).send().await?;
        ensure!(
            res.status().is_success(),
            "Gamma API error: {}",
            res.status()
        );
        let text = res.text().await?;

        match serde_json::from_str::<Market>(&text) {
            Ok(market) => Ok(market),
            Err(e) => {
                error!("Serde error: {:?}", e);
                Err(e.into())
            }
        }
    }

    // --------------------------------------
    // EVENTS
    // --------------------------------------

    /// GET /events/slug/{slug}
    pub async fn get_event_by_slug(&self, slug: &str) -> Result<RawEvent> {
        let url = format!("{}/{}{}", GAMMA_URL, EVENTS_SLUG, slug);

        let res = self.http.get(&url).send().await?;
        ensure!(
            res.status().is_success(),
            "Gamma API error: {}",
            res.status()
        );

        Ok(res.json::<RawEvent>().await?)
    }

    pub async fn get_event_by_id(&self, id: &str) -> Result<RawEvent> {
        let url = format!("{}/events?tag_id={}", GAMMA_URL, id);

        debug!("GET {}", url);
        let res = self.http.get(&url).send().await?;

        ensure!(
            res.status().is_success(),
            "Gamma API error: {}",
            res.status()
        );
        let text = res.text().await?;

        match serde_json::from_str::<RawEvent>(&text) {
            Ok(raw_event) => Ok(raw_event),
            Err(e) => {
                error!("Serde error: {:?}", e);
                Err(e.into())
            }
        }
    }

    pub async fn get_series_by_slug(&self, slug: &str) -> Result<Vec<Serie>> {
        let url = format!("{}/series?slug={}", GAMMA_URL, slug);

        let res = self.http.get(&url).send().await?;
        ensure!(
            res.status().is_success(),
            "Gamma API error: {}",
            res.status()
        );
        let text = res.text().await?;

        match serde_json::from_str::<Vec<Serie>>(&text) {
            Ok(series) => {
                debug!("Successfully parsed {} series", series.len());
                Ok(series)
            }
            Err(e) => {
                error!(
                    "Failed to parse JSON ({}). First {} chars: {}",
                    e,
                    ERROR_PREVIEW_CHARS,
                    &text.chars().take(ERROR_PREVIEW_CHARS).collect::<String>()
                );
                Err(e.into())
            }
        }
    }

    // --------------------------------------
    // Paginated queries used heavily by Polymarket
    // --------------------------------------

    pub async fn get_markets_paginated(
        &self,
        limit: u32,
        offset: u32,
        closed: Option<bool>,
        tag_id: Option<u64>,
        ascending: Option<bool>,
    ) -> Result<Vec<Market>> {
        let url = format!("{}/markets", GAMMA_URL);

        let mut req = self
            .http
            .get(&url)
            .query(&[("limit", limit), ("offset", offset)]);

        if let Some(c) = closed {
            req = req.query(&[("closed", c)]);
        }
        if let Some(tag_id) = tag_id {
            req = req.query(&[("tag_id", tag_id)]);
        }

        if let Some(a) = ascending {
            req = req.query(&[("ascending", a)]);
        }

        let res = req.send().await?;
        ensure!(
            res.status().is_success(),
            "Gamma API error: {}",
            res.status()
        );
        let text = res.text().await?;

        match serde_json::from_str::<Vec<Market>>(&text) {
            Ok(markets) => {
                debug!("Successfully parsed {} markets", markets.len());
                Ok(markets)
            }
            Err(e) => {
                error!(
                    "Failed to parse JSON ({}). First {} chars: {}",
                    e,
                    ERROR_PREVIEW_CHARS,
                    &text.chars().take(ERROR_PREVIEW_CHARS).collect::<String>()
                );
                Err(e.into())
            }
        }
    }

    pub async fn get_latest_events(
        &self,
        limit: Option<u32>,
        offset: Option<u32>,
        closed: Option<bool>,
        tag_id: Option<u64>,
        recurrence: Option<String>,
    ) -> Result<HashMap<String, RawEvent>> {
        let url = format!("{}/events", GAMMA_URL);

        let mut req = self.http.get(&url);

        if let Some(limit) = limit {
            req = req.query(&[("limit", limit)]);
        }
        if let Some(offset) = offset {
            req = req.query(&[("offset", offset)]);
        }
        if let Some(closed) = closed {
            req = req.query(&[("closed", closed)]);
        }
        if let Some(tag_id) = tag_id {
            req = req.query(&[("tag_id", tag_id)]);
        }
        if let Some(recurrence) = recurrence {
            req = req.query(&[("recurrence", recurrence)]);
        }

        let res = req.send().await?;
        ensure!(
            res.status().is_success(),
            "Gamma API error: {}",
            res.status()
        );

        let events: Vec<RawEvent> = res.json().await?;

        // Index by event id.
        let events_map: HashMap<String, RawEvent> = events
            .into_iter()
            .map(|event| (event.id.clone(), event))
            .collect();

        Ok(events_map)
    }

    pub async fn get_active_events_from_series(
        &self,
        series_slug: &str,
    ) -> Result<HashMap<String, RawEvent>> {
        // 1. Fetch the full series.
        let series = self.get_series_by_slug(series_slug).await?;

        let serie = series
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Serie '{}' not found", series_slug))?;

        let events = serie.events.unwrap_or_default();

        // 3. Build a map of active, non-closed events (id -> event).
        let mut active_map: HashMap<String, RawEvent> = HashMap::new();

        for event in events {
            if event.closed == Some(false) && event.active == Some(true) {
                let id = event.id.clone();
                active_map.entry(id).or_insert(event);
            }
        }
        Ok(active_map)
    }

    pub async fn get_active_markets_from_events(
        &self,
        events: &HashMap<String, RawEvent>,
    ) -> Result<HashMap<String, Vec<(String, Market)>>> {
        let mut active_markets: HashMap<String, Vec<(String, Market)>> = HashMap::new();

        for (id, event) in events {
            let slug = event.get_slug()?;
            info!("Fetching market for slug: {}", slug);

            let market = self.get_market_by_slug(slug).await?;

            if market.closed == Some(false)
                && market.active == Some(true)
                && market.accepting_orders == Some(true)
            {
                active_markets
                    .entry(id.clone())
                    .or_default()
                    .push((market.id.clone(), market.clone()));
            }
        }

        Ok(active_markets)
    }

    pub async fn get_all_active_events(&self) -> Result<HashMap<String, RawEvent>> {
        let mut all_events = HashMap::new();
        let mut offset = 0;

        loop {
            let url = format!("{}/events", GAMMA_URL);

            let res = self
                .http
                .get(&url)
                .query(&[
                    ("closed", "false"),
                    ("limit", &LIMIT_EVENTS.to_string()),
                    ("offset", &offset.to_string()),
                    ("order", "id"),
                    ("ascending", "false"),
                ])
                .send()
                .await?;

            if !res.status().is_success() {
                anyhow::bail!("Gamma API error: {}", res.status());
            }

            let events: Vec<RawEvent> = res.json().await?;

            let len = events.len();
            for event in events {
                all_events.insert(event.id.clone(), event);
            }

            info!("Fetched {} events (total: {})", len, all_events.len());

            // Fewer results than the page limit means we reached the end.
            if len < LIMIT_EVENTS as usize {
                break;
            }

            offset += LIMIT_EVENTS;
        }

        info!("Fetched all active events: {}", all_events.len());

        Ok(all_events)
    }

    pub async fn fetch_current_events_update(
        &self,
        current_events: &HashMap<String, Event>,
    ) -> Result<HashMap<String, RawEvent>> {
        let mut updated_events = HashMap::new();
        for id in current_events.keys() {
            let raw_event = self.get_event_by_id(id).await?;

            debug!("Updating event ID {}: {:?}", id, raw_event);
            updated_events.insert(id.clone(), raw_event);
        }

        Ok(updated_events)
    }

    pub async fn fetch_current_markets_update(
        &self,
        current_markets: &HashMap<String, Market>,
    ) -> Result<HashMap<String, Market>> {
        let mut updated_markets = HashMap::new();
        for id in current_markets.keys() {
            let market = self.get_market_by_id(id).await?;

            updated_markets.insert(id.clone(), market);
        }

        Ok(updated_markets)
    }
}

// These tests hit the live Gamma REST API, so they are non-deterministic and
// require network access. They are marked `#[ignore]` to keep `cargo test`
// green offline; run them explicitly with `cargo test -- --ignored`.
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires live Gamma API"]
    async fn test_get_markets() {
        let client = GammaClient::new();

        let _ = client.get_markets().await;
    }

    #[tokio::test]
    #[ignore = "requires live Gamma API"]
    async fn test_get_market_slug() {
        let client = GammaClient::new();
        let slug = "btc-updown-15m-1763407800";
        let market = client.get_market_by_slug(slug).await.unwrap();

        println!("market: {:?}", market);
    }

    #[tokio::test]
    #[ignore = "requires live Gamma API"]
    async fn test_get_market_id() {
        let client = GammaClient::new();
        let id = "83039";
        let market = client.get_market_by_id(id).await.unwrap();

        println!("market: {:?}", market);
    }

    #[tokio::test]
    #[ignore = "requires live Gamma API"]
    async fn test_get_all_events() {
        let client = GammaClient::new();

        let events = client
            .get_all_active_events()
            .await
            .expect("failed to get latest series");

        println!("Total events fetched: {}\n", events.len());

        let mut btc_15m_count = 0;
        let mut btc_15m_examples = Vec::new();

        for event in events.values() {
            let slug = match &event.slug {
                Some(s) if !s.is_empty() => s.to_lowercase(),
                _ => continue,
            };

            if slug.contains("btc-updown-15m") || slug.contains("btc-up-or-down-15m") {
                btc_15m_count += 1;
                if btc_15m_examples.len() < 5 {
                    btc_15m_examples
                        .push((event.id.clone(), event.slug.clone().unwrap_or_default()));
                }

                println!("BTC 15m found:");
                println!("  id    : {}", event.id);
                println!("  slug  : {}", event.slug.as_deref().unwrap_or("N/A"));
                println!(
                    "  markets : {:?}\n",
                    event.markets.as_ref().map(|m| m.len())
                );

                if let Some(ref markets) = event.markets {
                    for market in markets {
                        println!(
                            "  -> Market ID: {} | clob_token_ids: {:?}",
                            market.id, market.clob_token_ids
                        );
                    }
                }
            }
        }

        println!("--------------------------------");
        println!("SUMMARY");
        println!("--------------------------------");
        println!("Total active events          : {}", events.len());
        println!("BTC up/down 15m detected     : {}", btc_15m_count);
        if !btc_15m_examples.is_empty() {
            println!("Examples:");
            for (id, slug) in btc_15m_examples {
                println!("  - {} -> {}", id, slug);
            }
        }
        println!("--------------------------------");
    }

    #[tokio::test]
    #[ignore = "requires live Gamma API"]
    async fn test_get_actives_markets() {
        let client = GammaClient::new();

        let events = client
            .get_active_events_from_series("btc-up-or-down-15m")
            .await
            .expect("failed to get latest series");

        println!("events len : {:?}", events.len());

        for (_id, event) in events {
            println!("event id : {:?}", event.id);
            println!("event slug : {:?}", event.slug);
            println!("event markets : {:?}", event.markets);
        }
    }
}
