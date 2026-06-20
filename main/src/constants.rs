use std::time::Duration;

/// Grace period to let spawned tasks wind down after they are aborted.
pub const SHUTDOWN_GRACE_PERIOD: Duration = Duration::from_millis(500);
