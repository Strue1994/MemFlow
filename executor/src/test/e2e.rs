use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2ETestCase {
    pub name: String,
    pub description: String,
    pub steps: Vec<TestStep>,
    pub expected_result: TestResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStep {
    pub order: usize,
    pub action: TestAction,
    pub endpoint: String,
    pub request: Option<serde_json::Value>,
    pub expected_status: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TestAction {
    CreateWorkflow,
    ExecuteWorkflow,
    Optimize,
    Summarize,
    CreateTask,
    UpdateTask,
    AddEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub success: bool,
    pub assertions: Vec<Assertion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    pub field: String,
    pub expected: serde_json::Value,
    pub actual: Option<serde_json::Value>,
    pub passed: bool,
}

impl E2ETestCase {
    pub fn workflow_execution() -> Self {
        Self {
            name: "workflow_execution".to_string(),
            description: "端到端工作流执行测试: 创建 → 编译 → 执行".to_string(),
            steps: vec![
                TestStep {
                    order: 1,
                    action: TestAction::CreateWorkflow,
                    endpoint: "/compile".to_string(),
                    request: Some(serde_json::json!({
                        "n8n_json": {
                            "nodes": [{"id": "1", "name": "HTTP Request", "type": "httpRequest"}],
                            "connections": {}
                        },
                        "name": "test_workflow"
                    })),
                    expected_status: 200,
                },
                TestStep {
                    order: 2,
                    action: TestAction::ExecuteWorkflow,
                    endpoint: "/execute".to_string(),
                    request: Some(serde_json::json!({
                        "workflow_id": "{{workflow_id}}"
                    })),
                    expected_status: 200,
                },
            ],
            expected_result: TestResult {
                success: true,
                assertions: vec![],
            },
        }
    }

    pub fn optimize_learning() -> Self {
        Self {
            name: "optimize_learning".to_string(),
            description: "参数优化学习测试: 执行 → 获取统计 → 生成优化建议".to_string(),
            steps: vec![
                TestStep {
                    order: 1,
                    action: TestAction::ExecuteWorkflow,
                    endpoint: "/execute".to_string(),
                    request: Some(serde_json::json!({
                        "workflow_id": "test_wf"
                    })),
                    expected_status: 200,
                },
                TestStep {
                    order: 2,
                    action: TestAction::Optimize,
                    endpoint: "/optimize".to_string(),
                    request: Some(serde_json::json!({
                        "workflow_id": "test_wf"
                    })),
                    expected_status: 200,
                },
            ],
            expected_result: TestResult {
                success: true,
                assertions: vec![],
            },
        }
    }

    pub fn task_lifecycle() -> Self {
        Self {
            name: "task_lifecycle".to_string(),
            description: "任务生命周期测试: 创建 → 运行 → 完成".to_string(),
            steps: vec![
                TestStep {
                    order: 1,
                    action: TestAction::CreateTask,
                    endpoint: "/tasks".to_string(),
                    request: Some(serde_json::json!({
                        "workflow_id": "test_wf",
                        "owner": "test_user"
                    })),
                    expected_status: 201,
                },
                TestStep {
                    order: 2,
                    action: TestAction::UpdateTask,
                    endpoint: "/tasks/{{task_id}}".to_string(),
                    request: Some(serde_json::json!({
                        "status": "running"
                    })),
                    expected_status: 200,
                },
                TestStep {
                    order: 3,
                    action: TestAction::AddEvidence,
                    endpoint: "/tasks/{{task_id}}/evidence".to_string(),
                    request: Some(serde_json::json!({
                        "checkpoint": "step_1_complete",
                        "evidence": {"result": "success"}
                    })),
                    expected_status: 200,
                },
                TestStep {
                    order: 4,
                    action: TestAction::UpdateTask,
                    endpoint: "/tasks/{{task_id}}".to_string(),
                    request: Some(serde_json::json!({
                        "status": "done"
                    })),
                    expected_status: 200,
                },
            ],
            expected_result: TestResult {
                success: true,
                assertions: vec![],
            },
        }
    }
}
