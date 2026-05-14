use learning_engine::{
    Scheduler, ScheduleConfig,
    SafetyWhitelist, SafetyChecker,
    HyperoptAuto, HyperoptConfig,
    PromptOptimizer,
    ImpactAnalyzer, ImpactWorkflowMetrics as WorkflowMetrics,
};
use std::sync::Arc;
use std::collections::HashMap;
use reqwest::header::{HeaderMap, HeaderValue};

#[derive(Debug, Clone)]
struct WorkflowInsight {
    workflow_id: String,
    success_count: u64,
    failure_count: u64,
    avg_duration_ms: f64,
}

#[tokio::main]
async fn main() {
    println!("📚 MemFlow Learning Engine starting...");

    let executor_url = std::env::var("EXECUTOR_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    let executor_api_key = std::env::var("EXECUTOR_API_KEY").ok();
    let interval_seconds = std::env::var("LEARNING_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(3600);
    println!("📡 Executor URL: {}", executor_url);
    println!("🧠 Learning interval: {}s", interval_seconds);

    // Safety whitelist — must be created first, then injected into SafetyChecker
    let whitelist = Arc::new(SafetyWhitelist::new());
    let safety = Arc::new(SafetyChecker::new(whitelist));

    let hyperopt = Arc::new(HyperoptAuto::new(HyperoptConfig::default()));
    let prompt_opt = Arc::new(PromptOptimizer::new());
    let impact = Arc::new(ImpactAnalyzer::new(0.001, 50.0));

    // Scheduler drives the learning loop
    let config = ScheduleConfig {
        mode: learning_engine::scheduler::ScheduleMode::Interval { seconds: interval_seconds },
        enabled: true,
        max_concurrent_runs: 3,
    };
    let scheduler = Arc::new(Scheduler::new(config));

    println!("📅 Scheduler configured. Starting learning loop...");

    let safety_clone = safety.clone();
    let hyperopt_clone = hyperopt.clone();
    let prompt_opt_clone = prompt_opt.clone();
    let impact_clone = impact.clone();
    let executor_url_clone = executor_url.clone();
    let executor_api_key_clone = executor_api_key.clone();

    scheduler
        .start(move || {
            let safety = safety_clone.clone();
            let hyperopt = hyperopt_clone.clone();
            let prompt_opt = prompt_opt_clone.clone();
            let impact = impact_clone.clone();
            let executor_url = executor_url_clone.clone();
            let executor_api_key = executor_api_key_clone.clone();

            async move {
                println!("🔄 Running learning loop...");

                // Fetch real execution data from executor
                let insights = fetch_workflow_insights(&executor_url, executor_api_key.as_deref()).await;
                println!("📊 Analyzed {} workflows", insights.len());

                // Analyze workflow health
                for insight in &insights {
                    if insight.failure_count > insight.success_count / 4 {
                        println!("⚠️ Workflow {} has high failure rate: {}/{}",
                            insight.workflow_id, insight.failure_count, insight.success_count + insight.failure_count);
                    }
                }

                // Calculate overall metrics from real data
                let total_success: u64 = insights.iter().map(|i| i.success_count).sum();
                let total_failure: u64 = insights.iter().map(|i| i.failure_count).sum();
                let success_rate = if total_success + total_failure > 0 {
                    total_success as f64 / (total_success + total_failure) as f64
                } else {
                    1.0
                };
                let avg_duration = if !insights.is_empty() {
                    insights.iter().map(|i| i.avg_duration_ms).sum::<f64>() / insights.len() as f64
                } else {
                    0.0
                };

                println!("📈 Success rate: {:.1}%, Avg duration: {:.0}ms",
                    success_rate * 100.0, avg_duration);

                // Record metrics for hyperparameter optimization
                let metrics = learning_engine::hyperopt_auto::SystemMetrics {
                    success_rate,
                    avg_response_time_ms: avg_duration,
                    token_consumption: 0.0,
                    optimization_count: 0,
                    timestamp: chrono::Utc::now().timestamp(),
                };
                hyperopt.record_metrics(metrics).await;

                // --- Hyperparameter optimization ---
                match hyperopt.run_optimization().await {
                    Ok(Some(result)) => {
                        println!(
                            "📊 Hyperparameter optimization: expected_improvement={:.2}%",
                            result.expected_improvement * 100.0
                        );

                        let report = safety.check_workflow(&format!("{:?}", result.params)).await;
                        if safety.can_auto_approve(&report).await {
                            println!("✅ Auto-approved hyperparameter update");
                            let before = WorkflowMetrics::default();
                            let after = WorkflowMetrics {
                                success_rate: before.success_rate + result.expected_improvement,
                                ..before.clone()
                            };
                            impact
                                .record_optimization("hyperopt", 0, 1, &before, &after)
                                .await;
                        } else {
                            println!(
                                "⚠️ Requires manual review (blocked: {})",
                                report.blocked
                            );
                        }
                    }
                    Ok(None) => println!("ℹ️ No hyperparameter optimization needed"),
                    Err(e) => eprintln!("❌ Hyperopt error: {}", e),
                }

                // --- Prompt optimization ---
                let mut patterns = HashMap::new();
                for insight in &insights {
                    if insight.failure_count > 0 {
                        patterns.entry(insight.workflow_id.clone())
                            .or_insert_with(Vec::new);
                    }
                }
                let optims = prompt_opt.optimize_from_patterns(patterns).await;
                if !optims.is_empty() {
                    println!("📝 Generated {} prompt optimizations", optims.len());
                }

                println!("✅ Learning loop completed");
                Ok(())
            }
        })
        .await
        .unwrap_or_else(|e| eprintln!("Scheduler error: {}", e));
}

async fn fetch_workflow_insights(executor_url: &str, executor_api_key: Option<&str>) -> Vec<WorkflowInsight> {
    let mut headers = HeaderMap::new();
    if let Some(key) = executor_api_key.filter(|value| !value.trim().is_empty()) {
        if let Ok(value) = HeaderValue::from_str(key) {
            headers.insert("X-API-Key", value);
        }
    }
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let url = format!("{}/logs?limit=100", executor_url);

    match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                let empty = Vec::new();
                let logs = data.as_array().unwrap_or(&empty);
                let mut stats: HashMap<String, (u64, u64, f64, u64)> = HashMap::new();

                for log in logs {
                    let wf_id = log.get("workflow_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let error = log.get("error").and_then(|v| v.as_str());
                    let duration = log.get("duration_ms")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as f64;

                    let e = stats.entry(wf_id).or_insert((0, 0, 0.0, 0));
                    if error.is_some() {
                        e.1 += 1;
                    } else {
                        e.0 += 1;
                    }
                    e.2 += duration;
                    e.3 += 1;
                }

                return stats.into_iter()
                    .map(|(wf_id, (s, f, d, c))| WorkflowInsight {
                        workflow_id: wf_id,
                        success_count: s,
                        failure_count: f,
                        avg_duration_ms: d / c as f64,
                    })
                    .collect();
            }
        }
        Err(e) => println!("⚠️ Failed to fetch logs: {}", e),
    }
    Vec::new()
}
