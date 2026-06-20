use crate::constants::SHUTDOWN_GRACE_PERIOD;
use anyhow::Result;
use polymarket::manager::PolymarketManager;
use strategy::scout::Scout;
use tokio::signal;
use tokio::sync::mpsc;
use tracing::{error, info};

/// Wire up the manager and scout, run until Ctrl+C, then shut down cleanly.
pub async fn run() -> Result<()> {
    info!("Initializing the Polymarket Scout Bot");

    // Internal channels.
    let (wss_event_tx, wss_event_rx) = mpsc::unbounded_channel();
    let (market_update_tx, market_update_rx) = mpsc::unbounded_channel();

    // Build the manager (the core of the system).
    let manager = PolymarketManager::new(
        wss_event_tx,
        wss_event_rx,     // WS -> event listener
        market_update_tx, // notifies the scout
    )
    .await?;

    let scout_manager = manager.clone();
    let mut handles = manager.start();

    // Start the Scout (event-driven, low-latency).
    let mut scout = Scout::new(scout_manager, market_update_rx);
    let scout_handle = tokio::spawn(async move {
        scout.run().await;
        error!("Scout stopped unexpectedly!");
    });
    handles.push(scout_handle);

    // Run until the shutdown signal.
    info!("Bot running - press Ctrl+C to stop");
    signal::ctrl_c().await?;
    info!("Shutdown signal received - shutting down...");

    // Graceful shutdown: abort all tasks, then give them a moment to wind down.
    for handle in handles {
        handle.abort();
    }
    tokio::time::sleep(SHUTDOWN_GRACE_PERIOD).await;

    info!("Bot fully stopped.");
    Ok(())
}
