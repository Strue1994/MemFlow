# P2-4: Observability (Tracing, Alerts, Dashboards)

## Priority

P2 - Medium-term

## Key Files / Modules

- `executor/src/tracing.rs`
- `executor/src/metrics.rs`
- `agent-service/src/tracing.ts`
- `learning-engine/src/tracing.rs`
- `prometheus/alerts.yml`
- `grafana/dashboards/memflow.json`

## Goals

为系统增加分布式追踪、基于 Prometheus 的智能告警规则，并提供预置的 Grafana 仪表盘。

## Specific Requirements

### 1. Distributed Tracing (OpenTelemetry)

- 在 `executor`, `agent-service`, `learning-engine` 中集成 OpenTelemetry
- 生成 `trace_id` 并在服务间透传
- 导出到 Jaeger

```rust
// executor/src/tracing.rs
use opentelemetry::{global, sdk::propagators::TraceContextPropagator};
use opentelemetry_jaeger::Exporter;

pub fn init_tracing(service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let propagator = TraceContextPropagator::new();
    global::set_propagator(propagator);
    
    let exporter = Exporter::builder()
        .with_agent_endpoint("http://jaeger:14268/api/traces")
        .install()?;
    
    let tracer = sdk::trace::Provider::builder()
        .with_exporter(exporter)
        .build();
    
    global::set_tracer_provider(tracer);
    Ok(())
}

pub fn trace_span<T>(name: &str, f: impl FnOnce() -> T) -> T {
    let tracer = global::tracer("memflow");
    let _span = tracer.start(name);
    f()
}
```

```typescript
// agent-service/src/tracing.ts
import { NodeSDK } from '@opentelemetry/sdk-node';
import { JaegerExporter } from '@opentelemetry/exporter-jaeger';
import { HttpInstrumentation } from '@opentelemetry/instrumentation-http';

const sdk = new NodeSDK({
  exporter: new JaegerExporter({
    endpoint: 'http://jaeger:14268/api/traces',
  }),
  instrumentations: [new HttpInstrumentation()],
});

sdk.start();
```

### 2. Prometheus Alert Rules

```yaml
# prometheus/alerts.yml
groups:
  - name: memflow
    rules:
      - alert: WorkflowFailureRateHigh
        expr: |
          sum(rate(memflow_workflow_executions_total{status="failed"}[5m])) 
          / sum(rate(memflow_workflow_executions_total[5m])) 
          > 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Workflow failure rate > 10%"
          
      - alert: LearningEngineStalled
        expr: |
          time() - memflow_learning_last_run_timestamp 
          > 3600 * 2
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Learning engine hasn't run for 2 hours"
          
      - alert: MemoryHubSlowQuery
        expr: |
          histogram_quantile(0.95, memflow_memory_search_duration_seconds) 
          > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Memory search p95 > 1s"
```

### 3. Grafana Dashboard

```json
{
  "dashboard": {
    "title": "MemFlow Overview",
    "panels": [
      {
        "title": "Workflow QPS",
        "type": "graph",
        "targets": [
          {
            "expr": "sum(rate(memflow_workflow_executions_total[1m]))",
            "legendFormat": "QPS"
          }
        ]
      },
      {
        "title": "Execution Latency (p50/p95/p99)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, memflow_execution_duration_seconds)",
            "legendFormat": "p50"
          },
          {
            "expr": "histogram_quantile(0.95, memflow_execution_duration_seconds)",
            "legendFormat": "p95"
          },
          {
            "expr": "histogram_quantile(0.99, memflow_execution_duration_seconds)",
            "legendFormat": "p99"
          }
        ]
      },
      {
        "title": "Memory Search Duration",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, memflow_memory_search_duration_seconds)",
            "legendFormat": "p95"
          }
        ]
      },
      {
        "title": "Learning Loop Status",
        "type": "stat",
        "targets": [
          {
            "expr": "memflow_learning_last_run_timestamp",
            "legendFormat": "Last Run"
          }
        ]
      }
    ]
  }
}
```

## Acceptance Criteria

- [ ] 部署 Jaeger 后能看到完整调用链
- [ ] Prometheus 触发告警时有通知
- [ ] 导入仪表盘后所有面板有数据
- [ ] trace_id 在服务间正确透传

## Implementation Notes

1. Add to executor/Cargo.toml:
   ```toml
   opentelemetry = "0.22"
   opentelemetry-jaeger = "2.0"
   ```

2. Add to agent-service/package.json:
   ```json
   "@opentelemetry/sdk-node": "^0.52",
   "@opentelemetry/exporter-jaeger": "^0.52",
   ```