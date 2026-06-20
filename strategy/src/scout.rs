use crate::constants::MIN_PROFIT_USD;
use anyhow::Result;
use polymarket::manager::PolymarketManager;
use rayon::prelude::*;
use std::{sync::Arc, time::Instant};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info};

#[derive(Debug)]
pub struct Scout {
    pub polymarket_manager: Arc<PolymarketManager>,
    market_update_recipient: UnboundedReceiver<String>,
}

impl Scout {
    pub fn new(
        manager: Arc<PolymarketManager>,
        market_update_recipient: UnboundedReceiver<String>,
    ) -> Self {
        Self {
            polymarket_manager: manager,
            market_update_recipient,
        }
    }

    pub async fn scan_single_market(&self, market_id: &str) -> Result<()> {
        // The whole snapshot-building logic lives in the manager.
        let t1 = Instant::now();
        let snapshot = match self
            .polymarket_manager
            .generate_single_binary_market_snapshot(market_id)
            .await
        {
            Ok(snap) => snap,
            Err(_) => return Ok(()), // not enough data, fail silently
        };
        info!(
            "[SCOUT] Scanned generate_single_binary_market_snapshot {} in {:?}",
            market_id,
            t1.elapsed()
        );

        // Check for an arbitrage opportunity.
        if snapshot.is_arbitrage_opportunity() {
            let profit = snapshot.estimated_profit_usd();
            let size = snapshot.arbitrage_volume_usd();

            // Minimum threshold to avoid log spam.
            if profit > MIN_PROFIT_USD {
                info!(
                    "[SCOUT] ARB on {}: ${:.2} profit | ${:.2} size",
                    market_id, profit, size
                );
                // TODO: self.opportunity_tx.send(opportunity)?;
            }
        } else {
            info!(
                "[SCOUT] No arbitrage opportunity on {}, best_ask_sum : {:?}",
                market_id,
                snapshot.best_ask_sum()
            );
        }

        Ok(())
    }

    pub async fn run(&mut self) {
        info!("[SCOUT] Starting event-driven scanner...");

        while let Some(market_id) = self.market_update_recipient.recv().await {
            // Scan the updated market immediately (~100us latency).
            info!("[SCOUT] Market update received for {}", market_id);
            if let Err(e) = self.scan_single_market(&market_id).await {
                error!("[SCOUT] Error scanning {}: {}", market_id, e);
            }
        }

        info!("[SCOUT] Event-driven scanner stopped");
    }

    pub async fn scan_opportunities_intra_binary_market(&self) -> Result<()> {
        let binary_markets = self
            .polymarket_manager
            .generate_binary_market_snapshots()
            .await?;

        // Scan every binary market in parallel with rayon.
        binary_markets.par_iter().for_each(|(market_id, snapshot)| {
            debug!("Analyzing market {}: {:?}", market_id, snapshot);
            if snapshot.is_arbitrage_opportunity() {
                let profit = snapshot.estimated_profit_usd();
                let size = snapshot.arbitrage_volume_usd();
                info!(
                    "[SCOUT] Arbitrage on market {}: estimated profit = ${:.2} with size = {:?}",
                    market_id, profit, size
                );
            }
        });

        Ok(())
    }
}
