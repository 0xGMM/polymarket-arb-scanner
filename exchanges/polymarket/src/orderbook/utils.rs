use crate::orderbook::constants::{SCALE, SCALE_F64};

/// Price stored as a fixed-point integer with 6 decimals: $1.234567 -> 1_234_567 u32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PriceU32(pub u32);

/// Size stored as a fixed-point integer with 6 decimals: 12.345678 -> 12_345_678 u32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SizeU32(pub u32);

impl PriceU32 {
    #[inline(always)]
    pub fn from_f64(price: f64) -> Option<Self> {
        if !price.is_finite() || price < 0.0 || price > (u32::MAX as f64) / SCALE as f64 {
            return None;
        }
        Some(PriceU32((price * SCALE as f64).round() as u32))
    }

    #[inline(always)]
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / SCALE as f64
    }
}

impl SizeU32 {
    #[inline(always)]
    pub fn from_f64(size: f64) -> Option<Self> {
        if !size.is_finite() || size < 0.0 {
            return None;
        }
        Some(SizeU32((size * SCALE_F64).round() as u32))
    }

    #[inline(always)]
    pub fn to_f64(self) -> f64 {
        self.0 as f64 / SCALE_F64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_round_trips_through_fixed_point() {
        let p = PriceU32::from_f64(0.123456).unwrap();
        assert_eq!(p.0, 123_456);
        assert!((p.to_f64() - 0.123456).abs() < 1e-9);
    }

    #[test]
    fn size_round_trips_through_fixed_point() {
        let s = SizeU32::from_f64(12.5).unwrap();
        assert_eq!(s.0, 12_500_000);
        assert!((s.to_f64() - 12.5).abs() < 1e-9);
    }

    #[test]
    fn rejects_negative_and_non_finite_values() {
        assert!(PriceU32::from_f64(-0.1).is_none());
        assert!(PriceU32::from_f64(f64::NAN).is_none());
        assert!(PriceU32::from_f64(f64::INFINITY).is_none());
        assert!(SizeU32::from_f64(-1.0).is_none());
        assert!(SizeU32::from_f64(f64::NAN).is_none());
    }

    #[test]
    fn price_rejects_values_above_u32_capacity() {
        // u32::MAX / SCALE is roughly 4294.96; anything past that overflows.
        assert!(PriceU32::from_f64(5000.0).is_none());
    }

    #[test]
    fn ordering_follows_numeric_value() {
        let lo = PriceU32::from_f64(0.10).unwrap();
        let hi = PriceU32::from_f64(0.20).unwrap();
        assert!(lo < hi);
    }
}
