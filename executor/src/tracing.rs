use opentelemetry::{
    global, sdk::propagators::TraceContextPropagator,
    trace::{Tracer, TracerProvider},
    KeyValue,
};
use opentelemetry_jaeger::Exporter;
use std::sync::Arc;
use tokio::sync::RwLock;

pub fn init_tracing(service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let propagator = TraceContextPropagator::new();
    global::set_propagator(propagator);

    let exporter = Exporter::builder()
        .with_agent_endpoint("http://jaeger:14268/api/traces")
        .install()?;

    let provider = TracerProvider::builder()
        .with_exporter(exporter)
        .build();

    global::set_tracer_provider(provider);

    println!("📊 Tracing initialized for {}", service_name);
    Ok(())
}

pub fn trace_span<T>(name: &str, f: impl FnOnce() -> T) -> T {
    let tracer = global::tracer("memflow");
    let _span = tracer.start(name);
    f()
}

pub async fn trace_async<T, Fut>(name: &str, f: impl FnOnce() -> Fut) -> T
where
    Fut: std::future::Future<Output = T>,
{
    let tracer = global::tracer("memflow");
    let _span = tracer.start(name);
    f().await
}

#[derive(Clone)]
pub struct MetricsCollector {
    workflow_executions: Arc<RwLock<std::collections::HashMap<String, u64>>>,
    workflow_failures: Arc<RwLock<std::collections::HashMap<String, u64>>>,
    execution_duration_ms: Arc<RwLock<Vec<u64>>>,
    active_workflows: Arc<RwLock<u64>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            workflow_executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            workflow_failures: Arc::new(RwLock::new(std::collections::HashMap::new())),
            execution_duration_ms: Arc::new(RwLock::new(Vec::new())),
            active_workflows: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn record_execution(&self, workflow_id: &str, success: bool, duration_ms: u64) {
        {
            let mut executions = self.workflow_executions.write().await;
            *executions.entry(workflow_id.to_string()).or_insert(0) += 1;
        }
        if !success {
            let mut failures = self.workflow_failures.write().await;
            *failures.entry(workflow_id.to_string()).or_insert(0) += 1;
        }
        {
            let mut durations = self.execution_duration_ms.write().await;
            durations.push(duration_ms);
            if durations.len() > 1000 {
                durations.remove(0);
            }
        }
    }

    pub async fn get_qps(&self, workflow_id: &str) -> f64 {
        let executions = self.workflow_executions.read().await;
        executions.get(workflow_id).copied().unwrap_or(0) as f64
    }

    pub async fn get_failure_rate(&self, workflow_id: &str) -> f64 {
        let executions = self.workflow_executions.read().await;
        let failures = self.workflow_failures.read().await;
        let total = executions.get(workflow_id).copied().unwrap_or(0);
        let failed = failures.get(workflow_id).copied().unwrap_or(0);
        if total == 0 {
            return 0.0;
        }
        failed as f64 / total as f64
    }

    pub async fn get_p95_duration(&self) -> f64 {
        let durations = self.execution_duration_ms.read().await;
        if durations.is_empty() {
            return 0.0;
        }
        let mut sorted = durations.clone();
        sorted.sort();
        let idx = (sorted.len() as f64 * 0.95) as usize;
        sorted[idx.min(sorted.len() - 1)] as f64
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

pub static METRICS_COLLECTOR: MetricsCollector = MetricsCollector::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            collector.record_execution("test_wf", true, 100).await;
            collector.record_execution("test_wf", false, 200).await;
            
            let qps = collector.get_qps("test_wf").await;
            assert_eq!(qps, 2.0);
            
            let failure_rate = collector.get_failure_rate("test_wf").await;
            assert!(failure_rate > 0.4 && failure_rate < 0.6);
        });
    }
}