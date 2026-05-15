use lazy_static::lazy_static;
use prometheus::{
    register_counter, register_gauge, register_histogram_vec, register_int_counter_vec, Counter,
    Gauge, HistogramOpts, HistogramVec, IntCounterVec,
};

lazy_static! {
    pub static ref WORKFLOW_CALLS: Counter =
        register_counter!("workflow_calls_total", "Total number of workflow calls").unwrap();
    pub static ref WORKFLOW_DURATION: HistogramVec = register_histogram_vec!(
        HistogramOpts::new("workflow_duration_seconds", "Workflow execution duration"),
        &["workflow_id"]
    )
    .unwrap();
    pub static ref ACTIVE_WORKFLOWS: Gauge = register_gauge!(
        "active_workflows",
        "Number of currently executing workflows"
    )
    .unwrap();
    pub static ref COMPILE_REQUESTS: Counter =
        register_counter!("compile_requests_total", "Total number of compile requests").unwrap();
    pub static ref HTTP_REQUESTS: IntCounterVec = register_int_counter_vec!(
        "http_requests_total",
        "Total number of HTTP requests",
        &["endpoint", "method", "status"]
    )
    .unwrap();
}

pub fn record_call(workflow_id: &str, duration_secs: f64) {
    WORKFLOW_CALLS.inc();
    WORKFLOW_DURATION
        .with_label_values(&[workflow_id])
        .observe(duration_secs);
}

pub fn inc_active() {
    ACTIVE_WORKFLOWS.inc();
}

pub fn dec_active() {
    ACTIVE_WORKFLOWS.dec();
}

pub fn record_compile() {
    COMPILE_REQUESTS.inc();
}

pub fn record_http_request(endpoint: &str, method: &str, status: &str) {
    HTTP_REQUESTS
        .with_label_values(&[endpoint, method, status])
        .inc();
}
