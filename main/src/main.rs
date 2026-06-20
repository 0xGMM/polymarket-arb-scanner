mod app;
mod constants;
mod telemetry;

use anyhow::Result;

#[tokio::main(flavor = "multi_thread", worker_threads = 64)]
async fn main() -> Result<()> {
    // Hold the guard for the whole program so file logs keep flushing.
    let _guard = telemetry::init_tracing();

    app::run().await
}
