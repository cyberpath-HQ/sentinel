use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize tracing with the specified verbosity level and output format.
///
/// This function sets up the tracing subscriber with appropriate filtering based on verbosity.
/// By default, only logs from the 'sentinel' crate are shown at INFO level or higher.
/// Verbosity levels increase the log level for the 'sentinel' crate.
///
/// # Arguments
/// * `json` - If true, output logs in JSON format; otherwise, use human-readable format.
/// * `verbose` - Verbosity level: 0 for INFO, 1 for DEBUG, 2+ for TRACE.
///
/// # Examples
/// ```rust,no_run
/// init_tracing(false, 0); // INFO level, human-readable
/// init_tracing(true, 1); // DEBUG level, JSON
/// ```
pub fn init_tracing(json: bool, verbose: u8) {
    let level = match verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    let filter = EnvFilter::new(format!("sentinel={},sentinel_cli={}", level, level));

    let registry = tracing_subscriber::registry().with(filter);

    if json {
        registry
            .with(fmt::layer().json().flatten_event(true))
            .init();
    } else {
        registry.with(fmt::layer()).init();
    }
}
