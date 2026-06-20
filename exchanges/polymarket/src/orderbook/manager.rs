use crate::clob::constants::{SIDE_BUY, SIDE_SELL};
use crate::clob::message::{BookSnapshotMessage, PriceChangeMessage};
use crate::orderbook::constants::MIN_ORDER_SIZE;
use crate::orderbook::orderbook::Orderbook;
use anyhow::Result;
use futures::future::join_all;
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Per-asset delta accumulated from a price-change message:
/// (bid updates, ask updates, orderbook hash).
type BookDelta = (Vec<(f64, f64)>, Vec<(f64, f64)>, String);

#[derive(Debug, Clone, Default)]
pub struct OrderbookManager {
    orderbooks: Arc<RwLock<HashMap<String, Arc<RwLock<Orderbook>>>>>,
}

impl OrderbookManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            orderbooks: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn handle_snapshot_message(&self, snapshot: BookSnapshotMessage) -> Result<()> {
        let asset_id = snapshot.asset_id.clone();

        let bids = snapshot
            .bids
            .iter()
            .filter_map(|level| {
                let price = level.price.parse::<f64>().ok()?;
                let size = level.size.parse::<f64>().ok()?;
                (size > MIN_ORDER_SIZE).then_some((price, size))
            })
            .collect::<Vec<_>>();

        let asks = snapshot
            .asks
            .iter()
            .filter_map(|level| {
                let price = level.price.parse::<f64>().ok()?;
                let size = level.size.parse::<f64>().ok()?;
                (size > MIN_ORDER_SIZE).then_some((price, size))
            })
            .collect::<Vec<_>>();

        let book = Orderbook::new(
            bids,
            asks,
            snapshot.timestamp,
            snapshot.asset_id,
            snapshot.hash,
            snapshot.market,
        );

        let mut map = self.orderbooks.write().await;
        map.insert(asset_id, Arc::new(RwLock::new(book)));

        Ok(())
    }

    pub async fn handle_price_change_message(&self, msg: PriceChangeMessage) -> Result<()> {
        let mut deltas: HashMap<String, BookDelta> = HashMap::new();

        for update in msg.price_changes {
            let Ok(price) = update.price.parse::<f64>() else {
                continue;
            };
            let Ok(size) = update.size.parse::<f64>() else {
                continue;
            };

            let (bids, asks, _hash) = deltas
                .entry(update.asset_id.clone())
                .or_insert_with(|| (Vec::new(), Vec::new(), update.hash.clone()));

            match update.side.as_str() {
                SIDE_BUY => bids.push((price, size)),
                SIDE_SELL => asks.push((price, size)),
                _ => {}
            }
        }

        for (asset_id, (delta_bids, delta_asks, hash)) in deltas {
            self.apply_delta_to_book(
                &asset_id,
                &delta_bids,
                &delta_asks,
                msg.timestamp.clone(),
                hash,
            )
            .await?;
        }

        Ok(())
    }

    pub async fn apply_delta_to_book(
        &self,
        asset_id: &str,
        delta_bids: &[(f64, f64)],
        delta_asks: &[(f64, f64)],
        timestamp: String,
        hash: String,
    ) -> Result<()> {
        let orderbooks = self.orderbooks.read().await;

        if let Some(book_arc) = orderbooks.get(asset_id) {
            let mut book = book_arc.write().await;
            book.apply_delta(delta_bids, delta_asks, timestamp, hash);
        }
        Ok(())
    }

    pub async fn get_snapshot_parallel(&self) -> HashMap<String, Orderbook> {
        let orderbooks = self.orderbooks.read().await;
        let books: Vec<_> = orderbooks
            .iter()
            .map(|(id, arc)| (id.clone(), arc.clone()))
            .collect();
        drop(orderbooks);

        let mut snapshot = HashMap::with_capacity(books.len());
        let mut tasks = FuturesUnordered::new();

        for (asset_id, book_arc) in books {
            tasks.push(tokio::spawn(async move {
                let book = book_arc.read().await;
                (asset_id, book.clone())
            }));
        }

        while let Some(Ok((asset_id, book))) = tasks.next().await {
            snapshot.insert(asset_id, book);
        }

        snapshot
    }

    pub async fn get_single_market_snapshot(
        &self,
        token_ids: &[String],
    ) -> Result<HashMap<String, Orderbook>> {
        // 1. Quickly clone the relevant Arcs.
        let arcs = {
            let orderbooks = self.orderbooks.read().await;
            token_ids
                .iter()
                .filter_map(|id| orderbooks.get(id).map(|arc| (id.clone(), arc.clone())))
                .collect::<Vec<_>>()
        };

        // 2. Fetch all orderbooks in parallel.
        let futures = arcs.into_iter().map(|(token_id, arc)| async move {
            let book = arc.read().await;
            (token_id, book.clone())
        });

        let results = join_all(futures).await;

        Ok(results.into_iter().collect())
    }

    // TODO: handle the desync case (resubscribe / resnapshot on hash mismatch).
}
