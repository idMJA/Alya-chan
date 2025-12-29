use tracing_subscriber::{filter::EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logger() {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,alya_chan=debug")),
        )
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
}
