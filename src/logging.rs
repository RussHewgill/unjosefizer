use tracing_log::LogTracer;
use tracing_subscriber::{prelude::*, registry::Registry, EnvFilter};

pub fn init_logs() {
    tracing_subscriber::fmt()
        .with_env_filter("unjosefizer=debug")
        // .with_env_filter("derp_learning=trace, derp_learning_test=trace")
        // .with_max_level(tracing::Level::DEBUG)
        .without_time()
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_level(true)
        .compact()
        .finish()
        .try_init()
        .unwrap();
}
