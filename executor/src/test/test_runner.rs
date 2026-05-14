use crate::test::e2e::{E2ETestCase, TestStep, TestAction, TestResult, Assertion};
use reqwest::Client;
use serde_json::Value;

pub struct TestRunner {
    client: Client,
    base_url: String,
}

impl TestRunner {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn run_test(&self, test_case: &E2ETestCase) -> TestResult {
        println!("Running E2E test: {}", test_case.name);
        
        let mut assertions = Vec::new();
        let mut context = std::collections::HashMap::new();

        for step in &test_case.steps {
            let result = self.execute_step(step, &context).await;
            
            if result.status != step.expected_status {
                assertions.push(Assertion {
                    field: format!("step_{}_status", step.order),
                    expected: Value::from(step.expected_status),
                    actual: Some(Value::from(result.status)),
                    passed: false,
                });
            } else {
                assertions.push(Assertion {
                    field: format!("step_{}_status", step.order),
                    expected: Value::from(step.expected_status),
                    actual: Some(Value::from(result.status)),
                    passed: true,
                });
            }

            if let Some(id) = result.workflow_id {
                context.insert("workflow_id".to_string(), id);
            }
        }

        let success = assertions.iter().all(|a| a.passed);
        TestResult { success, assertions }
    }

    async fn execute_step(&self, step: &TestStep, context: &std::collections::HashMap<String, String>) -> StepResult {
        let url = self.build_url(&step.endpoint, context);
        
        let request = if let Some(req) = &step.request {
            let mut req_str = serde_json::to_string(req).unwrap_or_default();
            for (key, value) in context {
                req_str = req_str.replace(&format!("{{{}}}", key), value);
            }
            Some(req_str)
        } else {
            None
        };

        let client = reqwest::Client::new();
        let response = match step.action {
            TestAction::CreateWorkflow | TestAction::Optimize | TestAction::Summarize | TestAction::CreateTask => {
                client.post(&url).json(&request.unwrap_or_default()).send().await
            }
            TestAction::ExecuteWorkflow | TestAction::UpdateTask | TestAction::AddEvidence => {
                client.post(&url).json(&request.unwrap_or_default()).send().await
            }
        }.unwrap_or_else(|e| panic!("Request failed: {}", e));

        let status = response.status().as_u16();
        let workflow_id = response.headers()
            .get("X-Workflow-Id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        StepResult { status, workflow_id }
    }

    fn build_url(&self, endpoint: &str, context: &std::collections::HashMap<String, String>) -> String {
        let mut url = endpoint.to_string();
        for (key, value) in context {
            url = url.replace(&format!("{{{}}}", key), value);
        }
        format!("{}{}", self.base_url, url)
    }
}

struct StepResult {
    status: u16,
    workflow_id: Option<String>,
}

pub async fn run_api_tests(base_url: &str) -> Vec<TestResult> {
    let runner = TestRunner::new(base_url);
    let tests = vec![
        E2ETestCase::workflow_execution(),
    ];

    let mut results = Vec::new();
    for test in tests {
        let result = runner.run_test(&test).await;
        results.push(result);
    }
    results
}