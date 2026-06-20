use crate::event::event::RawEvent;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Serie {
    pub id: String,
    pub ticker: Option<String>,
    pub slug: Option<String>,
    pub title: Option<String>,

    #[serde(rename = "seriesType")]
    pub series_type: Option<String>,

    pub recurrence: Option<String>, // important for filtering

    pub active: Option<bool>,
    pub closed: Option<bool>,
    pub archived: Option<bool>,
    pub new: Option<bool>,
    pub restricted: Option<bool>,
    pub volume: Option<f64>,
    pub liquidity: Option<f64>,

    #[serde(rename = "commentCount")]
    pub comment_count: Option<u32>,

    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,

    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
    pub events: Option<Vec<RawEvent>>,
}
