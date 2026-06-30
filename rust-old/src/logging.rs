/// Initialize tracing subscriber for goonj logging.
///
/// Set `GOONJ_LOG=debug` (or info, warn, error, trace) to control log level.
///
/// # Panics
///
/// Panics if a global tracing subscriber has already been set.
pub fn init() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_env("GOONJ_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

/// Try to initialize the tracing subscriber. Returns `Err` if a subscriber is already set.
pub fn try_init() -> std::result::Result<(), String> {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_env("GOONJ_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_init_does_not_panic() {
        // May return Err if another test already initialized, but should not panic
        let _ = try_init();
    }
}
