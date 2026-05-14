use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(true)
        .with_span_list(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}
