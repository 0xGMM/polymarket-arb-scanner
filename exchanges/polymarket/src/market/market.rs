use crate::constants::BINARY_OUTCOME_COUNT;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Market {
    pub id: String,
    pub question: Option<String>,
    pub slug: Option<String>,
    pub spread: Option<f64>,
    pub liquidity: Option<String>,
    pub volume: Option<String>,
    pub outcomes: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
    pub closed: Option<bool>,

    #[serde(default)]
    pub outcomes_formatted: Option<Vec<String>>,

    #[serde(default)]
    pub clob_token_ids_formatted: Option<Vec<String>>,

    #[serde(rename = "startDate")]
    pub start_date: Option<String>,

    #[serde(rename = "endDate")]
    pub end_date: Option<String>,

    #[serde(rename = "conditionId")]
    pub condition_id: Option<String>,

    #[serde(rename = "questionID")]
    pub question_id: Option<String>,

    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,

    #[serde(rename = "enableOrderBook")]
    pub enable_order_book: Option<bool>,

    #[serde(rename = "clobTokenIds")]
    pub clob_token_ids: Option<String>, // JSON string holding the two token IDs

    #[serde(rename = "volumeClob")]
    pub volume_clob: Option<f64>,

    #[serde(rename = "liquidityClob")]
    pub liquidity_clob: Option<f64>,

    #[serde(rename = "acceptingOrders")]
    pub accepting_orders: Option<bool>,

    #[serde(rename = "orderPriceMinTickSize")]
    pub order_price_min_tick_size: Option<f64>,

    #[serde(rename = "orderMinSize")]
    pub order_min_size: Option<f64>,

    #[serde(rename = "umaResolutionStatus")]
    pub uma_resolution_status: Option<String>, // may be absent until resolved
}

impl Market {
    pub fn get_slug(&self) -> Result<&str> {
        self.slug
            .as_deref()
            .ok_or_else(|| anyhow!("Market slug is missing"))
    }

    pub fn is_binary(&self) -> bool {
        self.outcomes_formatted
            .as_ref()
            .is_some_and(|v| v.len() == BINARY_OUTCOME_COUNT)
            && self
                .clob_token_ids_formatted
                .as_ref()
                .is_some_and(|v| v.len() == BINARY_OUTCOME_COUNT)
    }

    pub fn is_resolved(&self) -> bool {
        self.uma_resolution_status.is_some()
    }

    pub fn compute_outcomes_formatted(&mut self) {
        if self.outcomes_formatted.is_none()
            && let Some(raw) = &self.outcomes
            && let Ok(vec) = serde_json::from_str::<Vec<String>>(raw)
        {
            self.outcomes_formatted = Some(vec);
        }
    }

    pub fn compute_clob_token_ids_formatted(&mut self) {
        if self.clob_token_ids_formatted.is_none()
            && let Some(raw) = &self.clob_token_ids
            && let Ok(vec) = serde_json::from_str::<Vec<String>>(raw)
        {
            self.clob_token_ids_formatted = Some(vec);
        }
    }

    pub fn is_active_market(&self) -> bool {
        let is_open = self.active.unwrap_or(false) && !self.closed.unwrap_or(false);
        let accepts_orders = self.accepting_orders.unwrap_or(false);
        let enable_ob = self.enable_order_book.unwrap_or(false);

        if let Some(end_date_str) = &self.end_date
            && let Ok(end_dt) = DateTime::parse_from_rfc3339(end_date_str)
            && end_dt <= Utc::now()
        {
            return false;
        }

        is_open && accepts_orders && enable_ob
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_clob_token_ids_formatted_parses_json_array() {
        let mut market = Market {
            clob_token_ids: Some(r#"["111","222"]"#.to_string()),
            ..Default::default()
        };
        market.compute_clob_token_ids_formatted();
        assert_eq!(
            market.clob_token_ids_formatted,
            Some(vec!["111".to_string(), "222".to_string()])
        );
    }

    #[test]
    fn is_binary_requires_two_outcomes_and_two_tokens() {
        let market = Market {
            outcomes_formatted: Some(vec!["YES".into(), "NO".into()]),
            clob_token_ids_formatted: Some(vec!["111".into(), "222".into()]),
            ..Default::default()
        };
        assert!(market.is_binary());

        let not_binary = Market {
            outcomes_formatted: Some(vec!["YES".into()]),
            clob_token_ids_formatted: Some(vec!["111".into()]),
            ..Default::default()
        };
        assert!(!not_binary.is_binary());
    }

    #[test]
    fn is_resolved_tracks_uma_status() {
        let resolved = Market {
            uma_resolution_status: Some("resolved".into()),
            ..Default::default()
        };
        assert!(resolved.is_resolved());
        assert!(!Market::default().is_resolved());
    }

    #[test]
    fn active_market_needs_all_flags_and_a_future_end_date() {
        let market = Market {
            active: Some(true),
            closed: Some(false),
            accepting_orders: Some(true),
            enable_order_book: Some(true),
            end_date: Some("2999-01-01T00:00:00Z".into()),
            ..Default::default()
        };
        assert!(market.is_active_market());
    }

    #[test]
    fn expired_market_is_inactive() {
        let market = Market {
            active: Some(true),
            closed: Some(false),
            accepting_orders: Some(true),
            enable_order_book: Some(true),
            end_date: Some("2000-01-01T00:00:00Z".into()),
            ..Default::default()
        };
        assert!(!market.is_active_market());
    }
}
