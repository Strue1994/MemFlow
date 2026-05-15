pub mod concurrency;
pub mod db;
pub mod db_node;
pub mod environment;
pub mod file;
pub mod error;
pub mod http;
pub mod http_server;
pub mod logging;
pub mod metrics;
pub mod shmem;
pub mod workflow_registry;
pub mod auth;
pub mod cluster;
pub mod slack;
pub mod telegram;
pub mod google_sheets;
pub mod plugin;
pub mod plugin_api;

use environment::Environment;
use error::ExecError;
use serde_json::Value;
use http::execute_http_request;

pub use compiler::{Instruction, MathOp, Workflow, WorkflowNode, HttpMethod};
pub use concurrency::{ConcurrencyLimiter, CONCURRENCY_LIMITER};
pub use workflow_registry::{get_workflow, register_workflow, list_workflows, list_versions, rollback};
pub use plugin::{PluginManager, PLUGIN_MANAGER};

pub struct Executor {
    pub env: Environment,
    loop_stack: Vec<LoopContext>,
}

struct LoopContext {
    iterator_var: String,
    start: i64,
    end: i64,
    step: i64,
    body_start: usize,
    return_pc: usize,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
            loop_stack: Vec::new(),
        }
    }

    pub fn execute(&mut self, workflow: &Workflow) -> Result<Value, ExecError> {
        self.execute_with_timeout(workflow, None)
    }

    pub fn execute_dag(&mut self, workflow: &Workflow) -> Result<Value, ExecError> {
        if workflow.nodes.is_empty() {
            return Ok(Value::Null);
        }
        
        let sorted = workflow.topological_sort().map_err(|e| ExecError::MathError(e))?;
        let mut last_result = Value::Null;
        
        for node in sorted {
            last_result = self.execute_instructions(&node.instructions)?;
        }
        
        Ok(last_result)
    }

    fn execute_instructions(&mut self, instructions: &[Instruction]) -> Result<Value, ExecError> {
        const MAX_STEPS: usize = 10000;
        let start_time = std::time::Instant::now();
        let default_timeout = std::time::Duration::from_secs(30);
        
        let mut result = None;
        let mut pc: isize = 0;
        let mut steps = 0;

        loop {
            if start_time.elapsed() > default_timeout {
                return Err(ExecError::MathError("Execution timeout".to_string()));
            }
            
            steps += 1;
            if steps > MAX_STEPS {
                return Err(ExecError::MathError(
                    "Maximum execution steps exceeded".to_string(),
                ));
            }

            if pc < 0 || (pc as usize) >= instructions.len() {
                break;
            }

            let instruction = &instructions[pc as usize];
            match instruction {
                Instruction::SetVariable { name, value } => {
                    self.env.set(name, value.clone());
                    pc += 1;
                }
                Instruction::MathOp { op, lhs, rhs, output } => {
                    let lhs_val = self.env.get(lhs)?;
                    let rhs_val = self.env.get(rhs)?;
                    let lhs_num = to_number(lhs_val)?;
                    let rhs_num = to_number(rhs_val)?;
                    let res = match op {
                        MathOp::Add => lhs_num + rhs_num,
                        MathOp::Sub => lhs_num - rhs_num,
                        MathOp::Mul => lhs_num * rhs_num,
                        MathOp::Div => {
                            if rhs_num == 0.0 {
                                return Err(ExecError::MathError("Division by zero".to_string()));
                            }
                            lhs_num / rhs_num
                        }
                    };
                    self.env.set(output, Value::from(res));
                    pc += 1;
                }
                Instruction::HttpRequest { method, url, headers, body, output_var } => {
                    let result = execute_http_request(*method, url, headers, body)?;
                    self.env.set(output_var, result);
                    pc += 1;
                }
                Instruction::Return { value } => {
                    result = Some(self.env.get(value)?.clone());
                    break;
                }
                Instruction::Code { .. } => {
                    return Err(ExecError::HttpError("Code execution not available".to_string()));
                }
                Instruction::If { condition_var, then_label, else_label } => {
                    let cond_val = self.env.get(condition_var)?;
                    let condition = to_bool(cond_val)?;
                    pc = if condition {
                        *then_label as isize
                    } else {
                        *else_label as isize
                    };
                }
                Instruction::For { iterator_var, start, end, step, body_start, body_end } => {
                    self.env.set(iterator_var, Value::from(*start));
                    self.loop_stack.push(LoopContext {
                        iterator_var: iterator_var.clone(),
                        start: *start,
                        end: *end,
                        step: *step,
                        body_start: *body_start,
                        return_pc: *body_end,
                    });
                    pc = *body_start as isize;
                }
                Instruction::Label(n) => {
                    if let Some(ctx) = self.loop_stack.last() {
                        if *n as usize == ctx.return_pc {
                            let current = self.env.get(&ctx.iterator_var)
                                .ok()
                                .and_then(|v| v.as_i64())
                                .unwrap_or(ctx.start);
                            let next = current + ctx.step;
                            if (next <= ctx.end && ctx.step > 0) || (next >= ctx.end && ctx.step < 0) {
                                self.env.set(&ctx.iterator_var, Value::from(next));
                                pc = ctx.body_start as isize;
                            } else {
                                self.loop_stack.pop();
                                pc += 1;
                            }
                        } else {
                            pc += 1;
                        }
                    } else {
                        pc += 1;
                    }
                }
                Instruction::CallWorkflow { workflow_id, params, output_var } => {
                    let sub_wf = workflow_registry::get_workflow(workflow_id, None)
                        .ok_or_else(|| ExecError::HttpError(format!("Workflow '{}' not found", workflow_id)))?;
                    
                    let is_tail_call = pc >= (instructions.len() as isize) - 1;
                    
                    if is_tail_call {
                        for (key, val) in params {
                            self.env.set(key, Value::String(val.clone()));
                        }
                        return self.execute_dag(&sub_wf);
                    } else {
                        let mut sub_exec = Executor::new();
                        for (key, val) in params {
                            sub_exec.env.set(key, Value::String(val.clone()));
                        }
                        let sub_result = sub_exec.execute_dag(&sub_wf)?;
                        self.env.set(output_var, sub_result);
                        pc += 1;
                    }
                }
                Instruction::DBQuery { connection, query, params: query_params, output_var } => {
                    let result = db_node::execute_db_query(connection, query, query_params)?;
                    self.env.set(output_var, result);
                    pc += 1;
                }
                Instruction::ReadFile { path, output_var } => {
                    let content = file::read_file(path)?;
                    self.env.set(output_var, content);
                    pc += 1;
                }
                Instruction::WriteFile { path, content, append } => {
                    file::write_file(path, content, *append)?;
                    pc += 1;
                }
                Instruction::SendEmail { .. } => {
                    return Err(ExecError::HttpError("SendEmail not implemented".to_string()));
                }
                Instruction::CallWasm { .. } => {
                    return Err(ExecError::HttpError("CallWasm not implemented".to_string()));
                }
                Instruction::ScheduleCron { .. } => {
                    return Err(ExecError::HttpError("ScheduleCron not implemented".to_string()));
                }
                Instruction::Webhook { .. } => {
                    return Err(ExecError::HttpError("Webhook not implemented".to_string()));
                }
                Instruction::TransformJson { .. } => {
                    return Err(ExecError::HttpError("TransformJson not implemented".to_string()));
                }
                Instruction::QueuePublish { .. } => {
                    return Err(ExecError::HttpError("QueuePublish not implemented".to_string()));
                }
                Instruction::QueueConsume { .. } => {
                    return Err(ExecError::HttpError("QueueConsume not implemented".to_string()));
                }
                Instruction::CacheGet { .. } => {
                    return Err(ExecError::HttpError("CacheGet not implemented".to_string()));
                }
                Instruction::CacheSet { .. } => {
                    return Err(ExecError::HttpError("CacheSet not implemented".to_string()));
                }
                Instruction::SlackSend { channel, text, token } => {
                    let result = slack::execute_slack_send(channel, text, token)?;
                    self.env.set("slack_result", result);
                    pc += 1;
                }
                Instruction::TelegramSend { chat_id, text, bot_token } => {
                    let result = telegram::execute_telegram_send(chat_id, text, bot_token)?;
                    self.env.set("telegram_result", result);
                    pc += 1;
                }
                Instruction::AwsS3Upload { .. } => {
                    return Err(ExecError::HttpError("AwsS3Upload not implemented".to_string()));
                }
                Instruction::AwsS3Download { .. } => {
                    return Err(ExecError::HttpError("AwsS3Download not implemented".to_string()));
                }
                Instruction::GoogleSheetsRead { spreadsheet_id, range, access_token, output_var } => {
                    let result = google_sheets::execute_google_sheets_read(spreadsheet_id, range, access_token)?;
                    self.env.set(output_var, result);
                    pc += 1;
                }
                Instruction::GoogleSheetsWrite { spreadsheet_id, range, values, access_token } => {
                    let result = google_sheets::execute_google_sheets_write(spreadsheet_id, range, &values, access_token)?;
                    self.env.set("sheets_write_result", result);
                    pc += 1;
                }
                Instruction::GoogleSheetsAppend { spreadsheet_id, range, values, access_token } => {
                    let result = google_sheets::execute_google_sheets_append(spreadsheet_id, range, &values, access_token)?;
                    self.env.set("sheets_append_result", result);
                    pc += 1;
                }
                Instruction::GithubCreateIssue { .. } => {
                    return Err(ExecError::HttpError("GithubCreateIssue not implemented".to_string()));
                }
                Instruction::NotionCreatePage { .. } => {
                    return Err(ExecError::HttpError("NotionCreatePage not implemented".to_string()));
                }
                Instruction::NotionQueryDatabase { .. } => {
                    return Err(ExecError::HttpError("NotionQueryDatabase not implemented".to_string()));
                }
            }
        }

        result.ok_or(ExecError::InvalidReturn)
    }

    pub fn execute_with_timeout(&mut self, workflow: &Workflow, timeout_secs: Option<u64>) -> Result<Value, ExecError> {
        if workflow.nodes.is_empty() {
            return Ok(Value::Null);
        }
        
        let entry_node = workflow.nodes.iter()
            .find(|n| n.id == workflow.entry)
            .or_else(|| workflow.nodes.first());
            
        if let Some(node) = entry_node {
            self.execute_instructions_with_timeout(&node.instructions, timeout_secs)
        } else {
            Ok(Value::Null)
        }
    }

    fn execute_instructions_with_timeout(&mut self, instructions: &[Instruction], timeout_secs: Option<u64>) -> Result<Value, ExecError> {
        const MAX_STEPS: usize = 10000;
        let start_time = std::time::Instant::now();
        let default_timeout = std::time::Duration::from_secs(30);
        let timeout = timeout_secs.map(|s| std::time::Duration::from_secs(s)).unwrap_or(default_timeout);
        
        let mut result = None;
        let mut pc: isize = 0;
        let mut steps = 0;

        loop {
            if start_time.elapsed() > timeout {
                return Err(ExecError::MathError("Execution timeout".to_string()));
            }
            
            steps += 1;
            if steps > MAX_STEPS {
                return Err(ExecError::MathError(
                    "Maximum execution steps exceeded".to_string(),
                ));
            }

            if pc < 0 || (pc as usize) >= instructions.len() {
                break;
            }

            let instruction = &instructions[pc as usize];
            match instruction {
                Instruction::SetVariable { name, value } => {
                    self.env.set(name, value.clone());
                    pc += 1;
                }
                Instruction::MathOp { op, lhs, rhs, output } => {
                    let lhs_val = self.env.get(lhs)?;
                    let rhs_val = self.env.get(rhs)?;
                    let lhs_num = to_number(lhs_val)?;
                    let rhs_num = to_number(rhs_val)?;
                    let res = match op {
                        MathOp::Add => lhs_num + rhs_num,
                        MathOp::Sub => lhs_num - rhs_num,
                        MathOp::Mul => lhs_num * rhs_num,
                        MathOp::Div => {
                            if rhs_num == 0.0 {
                                return Err(ExecError::MathError("Division by zero".to_string()));
                            }
                            lhs_num / rhs_num
                        }
                    };
                    self.env.set(output, Value::from(res));
                    pc += 1;
                }
                Instruction::HttpRequest { method, url, headers, body, output_var } => {
                    let result = execute_http_request(*method, url, headers, body)?;
                    self.env.set(output_var, result);
                    pc += 1;
                }
                Instruction::Return { value } => {
                    result = Some(self.env.get(value)?.clone());
                    break;
                }
                Instruction::Code { .. } => {
                    return Err(ExecError::HttpError("Code execution not available".to_string()));
                }
                Instruction::If { condition_var, then_label, else_label } => {
                    let cond_val = self.env.get(condition_var)?;
                    let condition = to_bool(cond_val)?;
                    pc = if condition {
                        *then_label as isize
                    } else {
                        *else_label as isize
                    };
                }
                Instruction::For { iterator_var, start, end, step, body_start, body_end } => {
                    self.env.set(iterator_var, Value::from(*start));
                    self.loop_stack.push(LoopContext {
                        iterator_var: iterator_var.clone(),
                        start: *start,
                        end: *end,
                        step: *step,
                        body_start: *body_start,
                        return_pc: *body_end,
                    });
                    pc = *body_start as isize;
                }
                Instruction::Label(n) => {
                    if let Some(ctx) = self.loop_stack.last() {
                        if *n as usize == ctx.return_pc {
                            let current = self.env.get(&ctx.iterator_var)
                                .ok()
                                .and_then(|v| v.as_i64())
                                .unwrap_or(ctx.start);
                            let next = current + ctx.step;
                            if (next <= ctx.end && ctx.step > 0) || (next >= ctx.end && ctx.step < 0) {
                                self.env.set(&ctx.iterator_var, Value::from(next));
                                pc = ctx.body_start as isize;
                            } else {
                                self.loop_stack.pop();
                                pc += 1;
                            }
                        } else {
                            pc += 1;
                        }
                    } else {
                        pc += 1;
                    }
                }
                Instruction::CallWorkflow { workflow_id, params, output_var } => {
                    let sub_wf = workflow_registry::get_workflow(workflow_id, None)
                        .ok_or_else(|| ExecError::HttpError(format!("Workflow '{}' not found", workflow_id)))?;
                    
                    let is_tail_call = pc >= (instructions.len() as isize) - 1;
                    
                    if is_tail_call {
                        for (key, val) in params {
                            self.env.set(key, Value::String(val.clone()));
                        }
                        return self.execute_instructions_with_timeout(&sub_wf.nodes.first()
                            .map(|n| n.instructions.clone())
                            .unwrap_or_default(), timeout_secs);
                    } else {
                        let mut sub_exec = Executor::new();
                        for (key, val) in params {
                            sub_exec.env.set(key, Value::String(val.clone()));
                        }
                        let sub_result = sub_exec.execute_with_timeout(&sub_wf, timeout_secs)?;
                        self.env.set(output_var, sub_result);
                        pc += 1;
                    }
                }
                Instruction::DBQuery { connection, query, params: query_params, output_var } => {
                    let result = db_node::execute_db_query(connection, query, query_params)?;
                    self.env.set(output_var, result);
                    pc += 1;
                }
                Instruction::ReadFile { path, output_var } => {
                    let content = file::read_file(path)?;
                    self.env.set(output_var, content);
                    pc += 1;
                }
                Instruction::WriteFile { path, content, append } => {
                    file::write_file(path, content, *append)?;
                    pc += 1;
                }
                Instruction::SendEmail { .. } => {
                    return Err(ExecError::HttpError("SendEmail not implemented".to_string()));
                }
                Instruction::CallWasm { .. } => {
                    return Err(ExecError::HttpError("CallWasm not implemented".to_string()));
                }
                Instruction::ScheduleCron { .. } => {
                    return Err(ExecError::HttpError("ScheduleCron not implemented".to_string()));
                }
                Instruction::Webhook { .. } => {
                    return Err(ExecError::HttpError("Webhook not implemented".to_string()));
                }
                Instruction::TransformJson { .. } => {
                    return Err(ExecError::HttpError("TransformJson not implemented".to_string()));
                }
                Instruction::QueuePublish { .. } => {
                    return Err(ExecError::HttpError("QueuePublish not implemented".to_string()));
                }
                Instruction::QueueConsume { .. } => {
                    return Err(ExecError::HttpError("QueueConsume not implemented".to_string()));
                }
                Instruction::CacheGet { .. } => {
                    return Err(ExecError::HttpError("CacheGet not implemented".to_string()));
                }
                Instruction::CacheSet { .. } => {
                    return Err(ExecError::HttpError("CacheSet not implemented".to_string()));
                }
                Instruction::SlackSend { channel, text, token } => {
                    let result = slack::execute_slack_send(channel, text, token)?;
                    self.env.set("slack_result", result);
                    pc += 1;
                }
                Instruction::TelegramSend { chat_id, text, bot_token } => {
                    let result = telegram::execute_telegram_send(chat_id, text, bot_token)?;
                    self.env.set("telegram_result", result);
                    pc += 1;
                }
                Instruction::AwsS3Upload { .. } => {
                    return Err(ExecError::HttpError("AwsS3Upload not implemented".to_string()));
                }
                Instruction::AwsS3Download { .. } => {
                    return Err(ExecError::HttpError("AwsS3Download not implemented".to_string()));
                }
                Instruction::GoogleSheetsRead { spreadsheet_id, range, access_token, output_var } => {
                    let result = google_sheets::execute_google_sheets_read(spreadsheet_id, range, access_token)?;
                    self.env.set(output_var, result);
                    pc += 1;
                }
                Instruction::GoogleSheetsWrite { spreadsheet_id, range, values, access_token } => {
                    let result = google_sheets::execute_google_sheets_write(spreadsheet_id, range, &values, access_token)?;
                    self.env.set("sheets_write_result", result);
                    pc += 1;
                }
                Instruction::GoogleSheetsAppend { spreadsheet_id, range, values, access_token } => {
                    let result = google_sheets::execute_google_sheets_append(spreadsheet_id, range, &values, access_token)?;
                    self.env.set("sheets_append_result", result);
                    pc += 1;
                }
                Instruction::GithubCreateIssue { .. } => {
                    return Err(ExecError::HttpError("GithubCreateIssue not implemented".to_string()));
                }
                Instruction::NotionCreatePage { .. } => {
                    return Err(ExecError::HttpError("NotionCreatePage not implemented".to_string()));
                }
                Instruction::NotionQueryDatabase { .. } => {
                    return Err(ExecError::HttpError("NotionQueryDatabase not implemented".to_string()));
                }
            }
        }

        result.ok_or(ExecError::InvalidReturn)
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

fn to_number(v: &Value) -> Result<f64, ExecError> {
    match v {
        Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s
            .parse::<f64>()
            .map_err(|_| ExecError::MathError(format!("Cannot parse '{}' as number", s))),
        _ => Err(ExecError::MathError(
            "Expected number or string".to_string(),
        )),
    }
}

fn to_bool(v: &Value) -> Result<bool, ExecError> {
    match v {
        Value::Bool(b) => Ok(*b),
        Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) != 0.0),
        Value::String(s) => Ok(!s.is_empty()),
        _ => Err(ExecError::MathError(
            "Cannot convert to boolean".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Number;

    #[test]
    fn test_math_op() {
        let mut executor = Executor::new();
        let wf = Workflow::from_instructions(vec![
            Instruction::SetVariable {
                name: "a".to_string(),
                value: Value::Number(Number::from(2)),
            },
            Instruction::SetVariable {
                name: "b".to_string(),
                value: Value::Number(Number::from(3)),
            },
            Instruction::MathOp {
                op: MathOp::Add,
                lhs: "a".to_string(),
                rhs: "b".to_string(),
                output: "c".to_string(),
            },
            Instruction::Return {
                value: "c".to_string(),
            },
        ]);
        let result = executor.execute(&wf).unwrap();
        assert_eq!(to_number(&result).unwrap(), 5.0);
    }

    #[test]
    fn test_if_branch_true() {
        let mut executor = Executor::new();
        let wf = Workflow::from_instructions(vec![
            Instruction::SetVariable {
                name: "condition".to_string(),
                value: Value::Bool(true),
            },
            Instruction::If {
                condition_var: "condition".to_string(),
                then_label: 2,
                else_label: 4,
            },
            Instruction::SetVariable {
                name: "result".to_string(),
                value: Value::String("then".to_string()),
            },
            Instruction::Return {
                value: "result".to_string(),
            },
            Instruction::SetVariable {
                name: "result".to_string(),
                value: Value::String("else".to_string()),
            },
            Instruction::Return {
                value: "result".to_string(),
            },
        ]);
        let result = executor.execute(&wf).unwrap();
        assert_eq!(result, Value::String("then".to_string()));
    }

    #[test]
    fn test_if_branch_false() {
        let mut executor = Executor::new();
        let wf = Workflow::from_instructions(vec![
            Instruction::SetVariable {
                name: "condition".to_string(),
                value: Value::Bool(false),
            },
            Instruction::If {
                condition_var: "condition".to_string(),
                then_label: 2,
                else_label: 4,
            },
            Instruction::SetVariable {
                name: "result".to_string(),
                value: Value::String("then".to_string()),
            },
            Instruction::Return {
                value: "result".to_string(),
            },
            Instruction::SetVariable {
                name: "result".to_string(),
                value: Value::String("else".to_string()),
            },
            Instruction::Return {
                value: "result".to_string(),
            },
        ]);
        let result = executor.execute(&wf).unwrap();
        assert_eq!(result, Value::String("else".to_string()));
    }

    #[test]
    fn test_subworkflow_call() {
        let mut sub_executor = Executor::new();
        sub_executor.env.set("x", Value::from(5));
        let child_wf = Workflow::from_instructions(vec![
            Instruction::MathOp {
                op: MathOp::Mul,
                lhs: "x".to_string(),
                rhs: "x".to_string(),
                output: "result".to_string(),
            },
            Instruction::Return {
                value: "result".to_string(),
            },
        ]);
        let result = sub_executor.execute(&child_wf).unwrap();
        assert_eq!(to_number(&result).unwrap(), 25.0);
    }

    #[test]
    fn test_for_loop() {
        let mut executor = Executor::new();
        let wf = Workflow::from_instructions(vec![
            Instruction::SetVariable {
                name: "sum".to_string(),
                value: Value::from(0),
            },
            Instruction::For {
                iterator_var: "i".to_string(),
                start: 1,
                end: 3,
                step: 1,
                body_start: 2,
                body_end: 5,
            },
            Instruction::MathOp {
                op: MathOp::Add,
                lhs: "sum".to_string(),
                rhs: "i".to_string(),
                output: "sum".to_string(),
            },
            Instruction::Label(5),
            Instruction::Return {
                value: "sum".to_string(),
            },
        ]);
        let result = executor.execute(&wf).unwrap();
        assert_eq!(to_number(&result).unwrap(), 6.0);
    }

    #[test]
    fn test_dag_topological_sort() {
        let wf = Workflow {
            nodes: vec![
                WorkflowNode {
                    id: "a".to_string(),
                    instructions: vec![],
                    dependencies: vec![],
                },
                WorkflowNode {
                    id: "b".to_string(),
                    instructions: vec![],
                    dependencies: vec!["a".to_string()],
                },
                WorkflowNode {
                    id: "c".to_string(),
                    instructions: vec![],
                    dependencies: vec!["a".to_string()],
                },
                WorkflowNode {
                    id: "d".to_string(),
                    instructions: vec![],
                    dependencies: vec!["b".to_string(), "c".to_string()],
                },
            ],
            entry: "a".to_string(),
        };
        let sorted = wf.topological_sort().unwrap();
        let ids: Vec<_> = sorted.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_dag_circular_dependency() {
        let wf = Workflow {
            nodes: vec![
                WorkflowNode {
                    id: "a".to_string(),
                    instructions: vec![],
                    dependencies: vec!["b".to_string()],
                },
                WorkflowNode {
                    id: "b".to_string(),
                    instructions: vec![],
                    dependencies: vec!["a".to_string()],
                },
            ],
            entry: "a".to_string(),
        };
        let result = wf.topological_sort();
        assert!(result.is_err());
    }
}
