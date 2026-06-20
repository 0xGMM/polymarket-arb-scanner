pub const EVENTS_SLUG: &str = "/events/slug/";
pub const MARKETS_SLUG: &str = "/markets/slug/";
pub const GAMMA_URL: &str = "https://gamma-api.polymarket.com";
pub const LIMIT_EVENTS: u32 = 500;
/// Number of characters of a failed response body to include in error logs.
pub const ERROR_PREVIEW_CHARS: usize = 1000;
// PAGINATION : curl "https://gamma-api.polymarket.com/events?order=id&ascending=false&closed=false&limit=50&offset=50"
// TAG FILTERING : curl "https://gamma-api.polymarket.com/markets?tag_id=100381&closed=false&limit=25&offset=0"
