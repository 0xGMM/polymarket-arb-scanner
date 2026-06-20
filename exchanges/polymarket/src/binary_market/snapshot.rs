use crate::constants::SHARE_PAYOFF_USD;
use crate::orderbook::orderbook::Orderbook;

#[derive(Debug, Clone)]
pub struct BinaryMarketSnapshot {
    pub market_id: String,
    pub platform: String,
    pub asset_ids: [String; 2],
    pub outcomes: [String; 2],
    pub orderbooks: [Orderbook; 2],
}

impl BinaryMarketSnapshot {
    pub fn new(
        market_id: String,
        platform: String,
        asset_ids: [String; 2],
        outcomes: [String; 2],
        orderbooks: [Orderbook; 2],
    ) -> Self {
        Self {
            market_id,
            platform,
            asset_ids,
            outcomes,
            orderbooks,
        }
    }

    pub fn yes(&self) -> &Orderbook {
        &self.orderbooks[0]
    }
    pub fn no(&self) -> &Orderbook {
        &self.orderbooks[1]
    }

    #[inline(always)]
    pub fn is_arbitrage_opportunity(&self) -> bool {
        self.best_ask_sum() < SHARE_PAYOFF_USD
    }

    #[inline(always)]
    pub fn best_ask_sum(&self) -> f64 {
        self.yes().best_ask_price() + self.no().best_ask_price()
    }

    #[inline(always)]
    pub fn arbitrage_volume_usd(&self) -> f64 {
        self.yes().best_ask_size().min(self.no().best_ask_size())
    }

    #[inline(always)]
    pub fn estimated_profit_usd(&self) -> f64 {
        let volume = self.arbitrage_volume_usd();
        let profit_per_unit = SHARE_PAYOFF_USD - self.best_ask_sum();
        (volume * profit_per_unit).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a snapshot from the best ask (price, size) of each side.
    /// An empty `Vec` produces an empty book (best ask price = +inf).
    fn snapshot(yes_ask: Vec<(f64, f64)>, no_ask: Vec<(f64, f64)>) -> BinaryMarketSnapshot {
        let yes = Orderbook::new(
            vec![],
            yes_ask,
            "t".into(),
            "yes".into(),
            "h".into(),
            "m".into(),
        );
        let no = Orderbook::new(
            vec![],
            no_ask,
            "t".into(),
            "no".into(),
            "h".into(),
            "m".into(),
        );
        BinaryMarketSnapshot::new(
            "market".into(),
            "Polymarket".into(),
            ["yes".into(), "no".into()],
            ["YES".into(), "NO".into()],
            [yes, no],
        )
    }

    #[test]
    fn detects_arbitrage_when_ask_sum_below_one() {
        // 0.40 + 0.55 = 0.95 < 1.0 -> arbitrage.
        let snap = snapshot(vec![(0.40, 100.0)], vec![(0.55, 80.0)]);
        assert!(snap.is_arbitrage_opportunity());
        assert!((snap.best_ask_sum() - 0.95).abs() < 1e-9);
    }

    #[test]
    fn no_arbitrage_when_ask_sum_at_or_above_one() {
        // 0.60 + 0.45 = 1.05 -> no arbitrage, and profit clamps to 0.
        let snap = snapshot(vec![(0.60, 100.0)], vec![(0.45, 80.0)]);
        assert!(!snap.is_arbitrage_opportunity());
        assert_eq!(snap.estimated_profit_usd(), 0.0);
    }

    #[test]
    fn volume_is_the_smaller_of_both_sides() {
        let snap = snapshot(vec![(0.40, 100.0)], vec![(0.55, 80.0)]);
        assert_eq!(snap.arbitrage_volume_usd(), 80.0);
    }

    #[test]
    fn profit_is_edge_times_volume() {
        // edge = 1 - 0.95 = 0.05, volume = 80 -> profit = 4.0.
        let snap = snapshot(vec![(0.40, 100.0)], vec![(0.55, 80.0)]);
        assert!((snap.estimated_profit_usd() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn empty_book_is_never_an_opportunity() {
        // A missing side has best ask = +inf, so the sum can never be < 1.
        let snap = snapshot(vec![], vec![(0.55, 80.0)]);
        assert!(!snap.is_arbitrage_opportunity());
        assert_eq!(snap.estimated_profit_usd(), 0.0);
    }
}
