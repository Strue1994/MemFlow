use crate::db::WorkflowDb;
use crate::test::e2e::{E2ETestCase, TestResult};
use std::sync::Arc;

pub struct TestRunner {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl TestRunner {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn run(&self, test: E2ETestCase) -> TestResult {
        println!("Running E2E test: {}", test.name);
        
        let mut workflow_id: Option<String> = None;
        let mut task_id: Option<String> = None;
        let mut assertions = Vec::new();

        for step in test.steps {
            let endpoint = step.endpoint.clone();
            let url = format!("{}{}", self.base_url, endpoint);
            
            let mut request_body = step.request.clone();
            
            if let Some(ref body) = request_body {
                if let Some(obj) = body.as_object() {
                    let mut new_obj = obj.clone();
                    if let Some(wf_id) = workflow_id.take() {
                        new_obj.insert("workflow_id".to_string(), serde_json::json!(wf_id));
                    }
                    if let Some(tid) = task_id.take() {
                        new_obj.insert("task_id".to_string(), serde_json::json!(tid));
                    }
                    request_body = Some(serde_json::Value::Object(new_obj));
                }
            }

            let request = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json");
            
            let request = if let Some(body) = request_body {
                request.body(body.to_string())
            } else {
                request
            };

            let result = request.send().await;
            
            match result {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let passed = status == step.expected_status;
                    
                    let actual = response.json::<serde_json::Value>().await.ok();
                    
                    if passed {
                        if let Some(ref body) = request_body {
                            if body.get("workflow_id").is_some() {
                                if let Some(wf) = body.get("workflow_id").and_then(|v| v.as_str()) {
                                    workflow_id = Some(wf.to_string());
                                }
                            }
                            if body.get("task_id").is_some() {
                                if let Some(tid) = body.get("task_id").and_then(|v| v.as_str()) {
                                    task_id = Some(tid.to_string());
                                }
                            }
                        }
                        
                        if let Some(ref act) = actual {
                            if let Some(wf_id) = act.get("workflow_id").and_then(|v| v.as_str()) {
                                workflow_id = Some(wf_id.to_string());
                            }
                            if let Some(tid) = act.get("task_id").and_then(|v| v.as_str()) {
                                task_id = Some(tid.to_string());
                            }
                        }
                    }

                    assertions.push(crate::test::e2e::Assertion {
                        field: format!("step_{}_status", step.order),
                        expected: serde_json::json!(step.expected_status),
                        actual: Some(serde_json::json!(status)),
                        passed,
                    });

                    println!("  Step {}: {} -> {}", step.order, step.action.as_str(), if passed { "PASS" } else { "FAIL" });
                }
                Err(e) => {
                    assertions.push(crate::test::e2e::Assertion {
                        field: format!("step_{}", step.order),
                        expected: serde_json::json!(step.expected_status),
                        actual: Some(serde_json::json!(e.to_string())),
                        passed: false,
                    });
                    println!("  Step {}: {} -> ERROR: {}", step.order, step.action.as_str(), e);
                }
            }
        }

        let success = assertions.iter().all(|a| a.passed);
        TestResult { success, assertions }
    }

    pub async fn run_all(&self) -> Vec<(String, TestResult)> {
        let tests = vec![
            E2ETestCase::workflow_execution(),
            E2ETestCase::optimize_learning(),
            E2ETestCase::task_lifecycle(),
        ];

        let mut results = Vec::new();
        for test in tests {
            let result = self.run(test.clone()).await;
            results.push((test.name, result));
        }
        results
    }
}

pub async fn run_api_tests(api_key: &str, base_url: &str) {
    let runner = TestRunner::new(api_key.to_string(), base_url.to_string());
    let results = runner.run_all().await;
    
    println!("\n=== E2E Test Results ===");
    for (name, result) in results {
        println!("{}: {}", name, if result.success { "PASSED" } else { "FAILED" });
        for assertion in result.assertions {
            if !assertion.passed {
                println!("  - {}: expected {:?}, got {:?}", 
                    assertion.field, assertion.expected, assertion.actual);
            }
        }
    }
}