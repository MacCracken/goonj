/// Initialize tracing subscriber for goonj logging.
///
/// Set `GOONJ_LOG=debug` (or info, warn, error, trace) to control log level.
pub fn init() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_env("GOONJ_LOG")
        .unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
}
