//! One-time tracing initialisation.
use std::str::FromStr;

use tracing::level_filters::LevelFilter;

#[ctor::ctor]
fn init_tracing() {
    let tracing_level = std::env::var("CUDA_HOOK_LOG").unwrap_or_else(|_| "error".to_string());
    if let Ok(level) = tracing::Level::from_str(&tracing_level) {
        let _ = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(LevelFilter::DEBUG)
            .with_thread_ids(true)
            .with_target(false)
            .with_level(true)
            .try_init();
    }
}
