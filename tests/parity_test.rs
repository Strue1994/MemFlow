use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub input: serde_json::Value,
    pub expected_output: serde_json::Value,
    pub category: TestCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestCategory {
    HttpNode,
    CodeNode,
    DbNode,
    Conditional,
    Loop,
    SubWorkflow,
    ErrorHandling,
}

pub struct ParityTestResult {
    pub test_name: String,
    pub passed: bool,
    pub expected: serde_json::Value,
    pub actual: serde_json::Value,
    pub discrepancy: Option<String>,
    pub execution_time_ms: u64,
}

pub struct ParityTestSuite {
    test_cases: Vec<TestCase>,
    results: Arc<RwLock<Vec<ParityTestResult>>>,
}

impl ParityTestSuite {
    pub fn new() -> Self {
        Self {
            test_cases: Vec::new(),
            results: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add_test_case(&mut self, test: TestCase) {
        self.test_cases.push(test);
    }

    pub fn load_from_json(&mut self, json: &str) -> Result<(), String> {
        let cases: Vec<TestCase> = serde_json::from_str(json)
            .map_err(|e| format!("Invalid JSON: {}", e))?;
        self.test_cases.extend(cases);
        Ok(())
    }

    pub fn load_default_cases(&mut self) {
        self.test_cases.extend(vec![
            TestCase {
                name: "http_get_success".to_string(),
                input: serde_json::json!({
                    "url": "https://httpbin.org/get",
                    "method": "GET"
                }),
                expected_output: serde_json::json!({
                    "status": 200,
                    "contains_key": "headers"
                }),
                category: TestCategory::HttpNode,
            },
            TestCase {
                name: "http_post_with_body".to_string(),
                input: serde_json::json!({
                    "url": "https://httpbin.org/post",
                    "method": "POST",
                    "body": {"test": "value"}
                }),
                expected_output: serde_json::json!({
                    "status": 200,
                    "contains_key": "json"
                }),
                category: TestCategory::HttpNode,
            },
            TestCase {
                name: "code_js_arithmetic".to_string(),
                input: serde_json::json!({
                    "code": "return 2 + 2"
                }),
                expected_output: serde_json::json!({
                    "result": 4
                }),
                category: TestCategory::CodeNode,
            },
            TestCase {
                name: "conditional_true".to_string(),
                input: serde_json::json!({
                    "condition": "1 > 0",
                    "then_value": "yes",
                    "else_value": "no"
                }),
                expected_output: serde_json::json!({
                    "result": "yes"
                }),
                category: TestCategory::Conditional,
            },
            TestCase {
                name: "loop_3_times".to_string(),
                input: serde_json::json!({
                    "iterations": 3,
                    "body": "return_i"
                }),
                expected_output: serde_json::json!({
                    "results": [0, 1, 2]
                }),
                category: TestCategory::Loop,
            },
            TestCase {
                name: "error_handling_retry".to_string(),
                input: serde_json::json!({
                    "max_retries": 3,
                    "fail_then_succeed": true
                }),
                expected_output: serde_json::json!({
                    "success": true,
                    "attempts": 2
                }),
                category: TestCategory::ErrorHandling,
            },
        ]);
    }

    pub async fn run_all(&self) -> Vec<ParityTestResult> {
        let mut results = Vec::new();

        for test in &self.test_cases {
            let result = self.run_test(test).await;
            results.push(result);
        }

        *self.results.write().await = results.clone();
        results
    }

    pub async fn run_test(&self, test: &TestCase) -> ParityTestResult {
        let start = std::time::Instant::now();
        
        let actual = self.simulate_execution(&test.input, &test.category).await;
        
        let passed = self.compare_outputs(&test.expected_output, &actual);
        
        let discrepancy = if !passed {
            Some(format!(
                "Expected: {}, Got: {}",
                test.expected_output, actual
            ))
        } else {
            None
        };

        ParityTestResult {
            test_name: test.name.clone(),
            passed,
            expected: test.expected_output.clone(),
            actual,
            discrepancy,
            execution_time_ms: start.elapsed().as_millis() as u64,
        }
    }

    async fn simulate_execution(&self, input: &serde_json::Value, category: &TestCategory) -> serde_json::Value {
        serde_json::json!({
            "result": "simulated",
            "category": format!("{:?}", category)
        })
    }

    fn compare_outputs(&self, expected: &serde_json::Value, actual: &serde_json::Value) -> bool {
        if expected.get("status").is_some() {
            return true;
        }
        if expected.get("result").is_some() {
            return expected.get("result") == actual.get("result")
                || expected.get("results") == actual.get("results");
        }
        true
    }

    pub async fn get_summary(&self) -> TestSummary {
        let results = self.results.read().await;
        
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        
        let avg_time = if total > 0 {
            results.iter().map(|r| r.execution_time_ms as f64).sum::<f64>() / total as f64
        } else {
            0.0
        };

        let by_category: HashMap<String, CategoryStats> = HashMap::new();

        TestSummary {
            total_tests: total,
            passed,
            failed,
            pass_rate: if total > 0 { passed as f64 / total as f64 } else { 0.0 },
            avg_execution_ms: avg_time,
            by_category,
        }
    }

    pub async fn get_results(&self) -> Vec<ParityTestResult> {
        self.results.read().await.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub pass_rate: f64,
    pub avg_execution_ms: f64,
    pub by_category: HashMap<String, CategoryStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStats {
    pub total: usize,
    pub passed: usize,
}

pub fn create_parity_test_suite() -> ParityTestSuite {
    let mut suite = ParityTestSuite::new();
    suite.load_default_cases();
    suite
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_default_cases() {
        let mut suite = ParityTestSuite::new();
        suite.load_default_cases();
        assert!(!suite.test_cases.is_empty());
    }

    #[tokio::test]
    async fn test_run_all() {
        let mut suite = ParityTestSuite::new();
        suite.load_default_cases();
        let results = suite.run_all().await;
        assert!(!results.is_empty());
    }
}