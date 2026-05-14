/// P3.3: OpenTelemetry integration
/// Structured tracing with span hierarchy for agent execution

use tracing::{info, warn, error, span, Level};
use tracing_subscriber::{fmt, EnvFilter, prelude::*};

pub fn init_telemetry(service_name: &str) {
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(true);

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    info!(service = %service_name, "Telemetry initialized");
}

/// Create a tracing span for an agent execution cycle
pub fn agent_span(task_id: &str) -> tracing::Span {
    span!(Level::INFO, "agent_cycle", task_id = %task_id)
}

/// Create a tracing span for a tool call
pub fn tool_span(tool_name: &str) -> tracing::Span {
    span!(Level::DEBUG, "tool_call", tool = %tool_name)
}

/// Create a tracing span for LLM completion
pub fn llm_span(model: &str) -> tracing::Span {
    span!(Level::DEBUG, "llm_completion", model = %model)
}
