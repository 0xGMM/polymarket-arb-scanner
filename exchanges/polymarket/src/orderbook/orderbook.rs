use crate::orderbook::utils::{PriceU32, SizeU32};
use std::collections::BTreeMap;
use tracing::trace;

/// In-memory orderbook for a single asset.
///
/// Fields are private on purpose: the cached `best_*` values must stay in sync
/// with the `bids`/`asks` maps, and that invariant is only upheld by `new` and
/// `apply_delta`. Read access goes through the accessor methods below.
#[derive(Debug, Clone)]
pub struct Orderbook {
    bids: BTreeMap<PriceU32, SizeU32>,
    asks: BTreeMap<PriceU32, SizeU32>,

    best_bid_price: f64,
    best_ask_price: f64,

    best_bid_size: f64,
    best_ask_size: f64,

    timestamp: String,
    asset_id: String,
    market: String,
    hash: String,
}

impl Orderbook {
    pub fn new(
        raw_bids: Vec<(f64, f64)>,
        raw_asks: Vec<(f64, f64)>,
        timestamp: String,
        asset_id: String,
        hash: String,
        market: String,
    ) -> Self {
        let mut bids = BTreeMap::new();
        let mut asks = BTreeMap::new();

        for &(p, s) in &raw_bids {
            if let (Some(price), Some(size)) = (PriceU32::from_f64(p), SizeU32::from_f64(s))
                && size.0 > 0
            {
                bids.insert(price, size);
            }
        }

        for &(p, s) in &raw_asks {
            if let (Some(price), Some(size)) = (PriceU32::from_f64(p), SizeU32::from_f64(s))
                && size.0 > 0
            {
                asks.insert(price, size);
            }
        }

        let best_bid_price = bids
            .iter()
            .next_back()
            .map(|(p, _)| p.to_f64())
            .unwrap_or(0.0);
        let best_ask_price = asks
            .iter()
            .next()
            .map(|(p, _)| p.to_f64())
            .unwrap_or(f64::INFINITY);

        let best_bid_size = bids
            .iter()
            .next_back()
            .map(|(_, s)| s.to_f64())
            .unwrap_or(0.0);
        let best_ask_size = asks.iter().next().map(|(_, s)| s.to_f64()).unwrap_or(0.0);

        Self {
            bids,
            asks,
            best_bid_price,
            best_ask_price,
            best_bid_size,
            best_ask_size,
            timestamp,
            asset_id,
            hash,
            market,
        }
    }

    // === Fast accessors ===

    #[inline(always)]
    pub fn best_ask_price(&self) -> f64 {
        self.best_ask_price
    }

    #[inline(always)]
    pub fn best_bid_price(&self) -> f64 {
        self.best_bid_price
    }

    #[inline(always)]
    pub fn best_ask_size(&self) -> f64 {
        self.best_ask_size
    }

    #[inline(always)]
    pub fn best_bid_size(&self) -> f64 {
        self.best_bid_size
    }

    /// True when the book has no orders on either side.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.bids.is_empty() && self.asks.is_empty()
    }

    // === Metadata accessors ===

    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    pub fn market(&self) -> &str {
        &self.market
    }

    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    // === Delta update ===
    pub fn apply_delta(
        &mut self,
        delta_bids: &[(f64, f64)],
        delta_asks: &[(f64, f64)],
        new_timestamp: String,
        new_hash: String,
    ) {
        // BIDS
        for &(raw_price, raw_size) in delta_bids {
            if let Some(price) = PriceU32::from_f64(raw_price) {
                if raw_size <= 0.0 {
                    self.bids.remove(&price);
                } else if let Some(size) = SizeU32::from_f64(raw_size) {
                    if size.0 > 0 {
                        self.bids.insert(price, size);
                    } else {
                        self.bids.remove(&price);
                    }
                }
            }
        }

        // ASKS
        for &(raw_price, raw_size) in delta_asks {
            if let Some(price) = PriceU32::from_f64(raw_price) {
                if raw_size <= 0.0 {
                    self.asks.remove(&price);
                } else if let Some(size) = SizeU32::from_f64(raw_size) {
                    if size.0 > 0 {
                        self.asks.insert(price, size);
                    } else {
                        self.asks.remove(&price);
                    }
                }
            }
        }

        // Recompute cached best levels (O(1)).
        self.best_bid_price = self
            .bids
            .iter()
            .next_back()
            .map(|(p, _)| p.to_f64())
            .unwrap_or(0.0);
        self.best_ask_price = self
            .asks
            .iter()
            .next()
            .map(|(p, _)| p.to_f64())
            .unwrap_or(f64::INFINITY);

        self.best_bid_size = self
            .bids
            .iter()
            .next_back()
            .map(|(_, s)| s.to_f64())
            .unwrap_or(0.0);
        self.best_ask_size = self
            .asks
            .iter()
            .next()
            .map(|(_, s)| s.to_f64())
            .unwrap_or(0.0);

        self.timestamp = new_timestamp;
        self.hash = new_hash;

        trace!(
            asset_id = %self.asset_id,
            best_bid = self.best_bid_price,
            best_ask = self.best_ask_price,
            "[ORDERBOOK] delta applied"
        );
    }

    // === Read accessors ===
    pub fn get_bids(&self) -> &BTreeMap<PriceU32, SizeU32> {
        &self.bids
    }

    pub fn get_asks(&self) -> &BTreeMap<PriceU32, SizeU32> {
        &self.asks
    }

    /// Cost to fill `amount_in` units by walking the book from the best price.
    /// Returns `None` if there is not enough liquidity to fill the whole size.
    pub fn get_total_cost(&self, mut amount_in: f64, is_buy: bool) -> Option<f64> {
        let mut total_cost = 0.0;
        let levels = if is_buy { &self.asks } else { &self.bids };

        for (price_u32, size_u32) in levels {
            let price = price_u32.to_f64();
            let size = size_u32.to_f64();
            let take = size.min(amount_in);
            total_cost += take * price;
            amount_in -= take;
            if amount_in <= 0.0 {
                break;
            }
        }

        if amount_in > 0.0 {
            None
        } else {
            Some(total_cost)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>) -> Orderbook {
        Orderbook::new(
            bids,
            asks,
            "t".into(),
            "asset".into(),
            "h".into(),
            "m".into(),
        )
    }

    #[test]
    fn new_computes_best_levels() {
        let ob = book(
            vec![(0.40, 10.0), (0.42, 5.0)],
            vec![(0.50, 8.0), (0.48, 3.0)],
        );
        assert_eq!(ob.best_bid_price(), 0.42);
        assert_eq!(ob.best_bid_size(), 5.0);
        assert_eq!(ob.best_ask_price(), 0.48);
        assert_eq!(ob.best_ask_size(), 3.0);
    }

    #[test]
    fn empty_book_has_infinite_best_ask() {
        let ob = book(vec![], vec![]);
        assert_eq!(ob.best_bid_price(), 0.0);
        assert_eq!(ob.best_ask_price(), f64::INFINITY);
    }

    #[test]
    fn zero_size_levels_are_dropped_on_construction() {
        let ob = book(vec![(0.40, 0.0)], vec![(0.50, 5.0)]);
        assert!(ob.get_bids().is_empty());
        assert_eq!(ob.best_ask_price(), 0.50);
    }

    #[test]
    fn apply_delta_updates_and_removes_levels() {
        let mut ob = book(vec![(0.40, 10.0)], vec![(0.50, 8.0)]);

        // New best ask at 0.49, and the old 0.50 level is removed (size 0).
        ob.apply_delta(&[], &[(0.49, 4.0), (0.50, 0.0)], "t2".into(), "h2".into());

        assert_eq!(ob.best_ask_price(), 0.49);
        assert_eq!(ob.best_ask_size(), 4.0);
        assert_eq!(ob.hash(), "h2");
    }

    #[test]
    fn total_cost_walks_multiple_ask_levels() {
        let ob = book(vec![], vec![(0.50, 2.0), (0.60, 5.0)]);
        // Buy 4 units: 2 @ 0.50 + 2 @ 0.60 = 1.0 + 1.2 = 2.2.
        let cost = ob.get_total_cost(4.0, true).unwrap();
        assert!((cost - 2.2).abs() < 1e-9);
    }

    #[test]
    fn total_cost_is_none_when_liquidity_is_insufficient() {
        let ob = book(vec![], vec![(0.50, 2.0)]);
        assert!(ob.get_total_cost(10.0, true).is_none());
    }
}
