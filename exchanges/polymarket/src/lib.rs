// A type-per-file layout (e.g. `market/market.rs`) is intentional here; the
// resulting `foo::foo` paths trip clippy's `module_inception` lint, which we
// allow project-wide rather than collapsing types into their `mod.rs`.
#![allow(clippy::module_inception)]

pub mod binary_market;
pub mod clob;
pub mod constants;
pub mod endpoints;
pub mod event;
pub mod gamma;
pub mod indexer;
pub mod manager;
pub mod market;
pub mod orderbook;
pub mod serie;
