use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Initialize tracing to stdout and a rolling hourly log file.
///
/// The returned [`WorkerGuard`] must be kept alive for the lifetime of the
/// program: dropping it stops the non-blocking file writer from flushing.
pub fn init_tracing() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::hourly("logs", "bot.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(std::io::stdout) // console
                .with_target(false)
                .with_thread_ids(true),
        )
        .with(
            fmt::layer()
                .with_writer(non_blocking) // file
                .with_ansi(false), // no colors in file logs
        )
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        ) // default level
        .init();

    guard
}
