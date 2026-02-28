use std::sync::Once;

static INIT: Once = Once::new();

pub fn init_tracing() {
    INIT.call_once(|| {
        // Default to 'error' to keep tests quiet unless RUST_LOG is set
        let filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("error"));

        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .try_init();
    });
}
