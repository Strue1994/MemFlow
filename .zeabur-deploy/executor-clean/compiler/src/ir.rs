use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MathOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl Default for HttpMethod {
    fn default() -> Self {
        HttpMethod::Get
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Instruction {
    HttpRequest {
        method: HttpMethod,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<Value>,
        output_var: String,
    },
    SetVariable {
        name: String,
        value: Value,
    },
    MathOp {
        op: MathOp,
        lhs: String,
        rhs: String,
        output: String,
    },
    Return {
        value: String,
    },
    Code {
        script: String,
        output_var: String,
    },
    If {
        condition_var: String,
        then_label: usize,
        else_label: usize,
    },
    For {
        iterator_var: String,
        start: i64,
        end: i64,
        step: i64,
        body_start: usize,
        body_end: usize,
    },
    Label(usize),
    CallWorkflow {
        workflow_id: String,
        params: Vec<(String, String)>,
        output_var: String,
    },
    DBQuery {
        connection: String,
        query: String,
        params: Vec<Value>,
        output_var: String,
    },
    ReadFile {
        path: String,
        output_var: String,
    },
    WriteFile {
        path: String,
        content: Value,
        append: bool,
    },
    SendEmail {
        to: String,
        subject: String,
        body: String,
        smtp_config: String,
    },
    CallWasm {
        module_id: String,
        function: String,
        params: Vec<Value>,
        output_var: String,
    },
    ScheduleCron {
        cron_expression: String,
        workflow_id: String,
    },
    Webhook {
        path: String,
        method: String,
        handler_workflow: String,
    },
    TransformJson {
        input_var: String,
        output_var: String,
        transformation: Value,
    },
    QueuePublish {
        queue_name: String,
        message: Value,
    },
    QueueConsume {
        queue_name: String,
        output_var: String,
    },
    CacheGet {
        key: String,
        output_var: String,
    },
    CacheSet {
        key: String,
        value: Value,
        ttl_seconds: Option<u64>,
    },
    SlackSend {
        channel: String,
        text: String,
        token: String,
    },
    TelegramSend {
        chat_id: String,
        text: String,
        bot_token: String,
    },
    AwsS3Upload {
        bucket: String,
        key: String,
        body: Value,
        region: String,
    },
    AwsS3Download {
        bucket: String,
        key: String,
        output_var: String,
    },
    GoogleSheetsRead {
        spreadsheet_id: String,
        range: String,
        access_token: String,
        output_var: String,
    },
    GoogleSheetsWrite {
        spreadsheet_id: String,
        range: String,
        values: Vec<Vec<Value>>,
        access_token: String,
    },
    GoogleSheetsAppend {
        spreadsheet_id: String,
        range: String,
        values: Vec<Vec<Value>>,
        access_token: String,
    },
    GithubCreateIssue {
        owner: String,
        repo: String,
        title: String,
        body: String,
        labels: Option<Vec<String>>,
        token: String,
    },
    NotionCreatePage {
        database_id: String,
        properties: Value,
        token: String,
    },
    NotionQueryDatabase {
        database_id: String,
        filter: Option<Value>,
        token: String,
        output_var: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub instructions: Vec<Instruction>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub nodes: Vec<WorkflowNode>,
    pub entry: String,
}

impl Workflow {
    pub fn from_instructions(instructions: Vec<Instruction>) -> Self {
        Self {
            nodes: vec![WorkflowNode {
                id: "main".to_string(),
                instructions,
                dependencies: vec![],
            }],
            entry: "main".to_string(),
        }
    }

    pub fn from_single_node(instructions: Vec<Instruction>) -> Self {
        Self::from_instructions(instructions)
    }

    pub fn topological_sort(&self) -> Result<Vec<&WorkflowNode>, String> {
        let mut in_degree: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut graph: std::collections::HashMap<String, &WorkflowNode> =
            std::collections::HashMap::new();

        for node in &self.nodes {
            in_degree.entry(node.id.clone()).or_insert(0);
            graph.entry(node.id.clone()).or_insert(node);
            for dep in &node.dependencies {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut sorted = Vec::new();
        while let Some(node_id) = queue.pop() {
            if let Some(node) = graph.get(&node_id) {
                sorted.push(*node);
                for dep in &node.dependencies {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(dep.clone());
                        }
                    }
                }
            }
        }

        if sorted.len() != self.nodes.len() {
            return Err("Circular dependency detected".to_string());
        }

        sorted.reverse();
        Ok(sorted)
    }
}

impl Workflow {
    pub fn new() -> Self {
        Self {
            nodes: vec![],
            entry: String::new(),
        }
    }

    pub fn with_entry(instructions: Vec<Instruction>, entry: &str) -> Self {
        Self {
            nodes: vec![WorkflowNode {
                id: entry.to_string(),
                instructions,
                dependencies: vec![],
            }],
            entry: entry.to_string(),
        }
    }
}

impl Default for Workflow {
    fn default() -> Self {
        Self::new()
    }
}
