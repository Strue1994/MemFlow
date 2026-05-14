use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenTestCase {
    pub name: String,
    pub workflow: Value,
    pub input: Option<Value>,
    pub expected_output: Option<Value>,
    pub tags: Vec<String>,
}

#[derive(Debug)]
pub struct TestResult {
    pub test_name: String,
    pub passed: bool,
    pub expected: Option<Value>,
    pub actual: Option<Value>,
    pub error: Option<String>,
}

pub struct TestRunner;

impl TestRunner {
    pub fn load_golden_tests(golden_dir: &Path) -> Vec<GoldenTestCase> {
        let mut tests = Vec::new();

        if !golden_dir.exists() {
            return tests;
        }

        if let Ok(entries) = std::fs::read_dir(golden_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(test) = serde_json::from_str::<GoldenTestCase>(&content) {
                            tests.push(test);
                        }
                    }
                }
            }
        }

        tests
    }

    pub fn run_test(
        &self,
        test: &GoldenTestCase,
        compiler: &crate::compiler::Compiler,
        executor: &crate::Executor,
    ) -> TestResult {
        let test_name = test.name.clone();

        let workflow_json = serde_json::to_string(&test.workflow).unwrap_or_default();

        let ir = match compiler.parse(&workflow_json) {
            Ok(ir) => ir,
            Err(e) => {
                return TestResult {
                    test_name,
                    passed: false,
                    expected: test.expected_output.clone(),
                    actual: None,
                    error: Some(format!("Parse error: {}", e)),
                };
            }
        };

        let result = executor.execute(ir, test.input.clone());

        match result {
            Ok(actual) => {
                let passed = if let Some(expected) = &test.expected_output {
                    Self::deep_equal(&actual, expected)
                } else {
                    true
                };

                TestResult {
                    test_name,
                    passed,
                    expected: test.expected_output.clone(),
                    actual: Some(actual),
                    error: if passed {
                        None
                    } else {
                        Some("Output mismatch".to_string())
                    },
                }
            }
            Err(e) => TestResult {
                test_name,
                passed: false,
                expected: test.expected_output.clone(),
                actual: None,
                error: Some(format!("Execution error: {}", e)),
            },
        }
    }

    fn deep_equal(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                let fa = na.as_f64().unwrap_or(0.0);
                let fb = nb.as_f64().unwrap_or(0.0);
                (fa - fb).abs() < 1e-6
            }
            (Value::Array(la), Value::Array(lb)) => {
                la.len() == lb.len()
                    && la
                        .iter()
                        .zip(lb.iter())
                        .all(|(a, b)| Self::deep_equal(a, b))
            }
            (Value::Object(oa), Value::Object(ob)) => {
                oa.len() == ob.len()
                    && oa
                        .iter()
                        .all(|(k, v)| ob.get(k).map_or(false, |vb| Self::deep_equal(v, vb)))
            }
            _ => a == b,
        }
    }

    pub fn generate_report(results: &[TestResult]) -> String {
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = results.len() - passed;

        let mut report = format!(
            "=== MemFlow Parity Test Report ===\n\nTotal: {} tests\nPassed: {}\nFailed: {}\n\n",
            results.len(),
            passed,
            failed
        );

        if failed > 0 {
            report.push_str("=== Failed Tests ===\n\n");
            for result in results.iter().filter(|r| !r.passed) {
                report.push_str(&format!("Test: {}\n", result.test_name));
                if let Some(error) = &result.error {
                    report.push_str(&format!("  Error: {}\n", error));
                }
                if let (Some(expected), Some(actual)) = (&result.expected, &result.actual) {
                    report.push_str(&format!("  Expected: {}\n", expected));
                    report.push_str(&format!("  Actual: {}\n", actual));
                }
                report.push('\n');
            }
        }

        report
    }
}
