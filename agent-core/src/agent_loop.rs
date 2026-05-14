/// P2.1: Rust-native Agent Loop (replaces TypeScript agent-loop.ts)
/// Think-Act-Observe cycle running directly in Rust, no HTTP overhead.

use crate::llm::{LLMMessage, LLMProvider, LLMResponse, OpenAIProvider};
use crate::tool::Tool;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model: String,
    pub provider: String,
    pub system_prompt: String,
    pub max_iterations: u32,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".into(),
            provider: "openai".into(),
            system_prompt: "You are a helpful AI agent. Complete the user's task efficiently.".into(),
            max_iterations: 10,
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub success: bool,
    pub output: String,
    pub iterations: u32,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
}

pub struct AgentLoop {
    config: AgentConfig,
    provider: Box<dyn LLMProvider>,
    tools: Vec<Box<dyn Tool>>,
    messages: Vec<AgentMessage>,
}

impl AgentLoop {
    pub fn new(config: AgentConfig, api_key: &str) -> Self {
        let provider: Box<dyn LLMProvider> = match config.provider.as_str() {
            "anthropic" => Box::new(crate::llm::AnthropicProvider::new(api_key, &config.model)),
            _ => Box::new(OpenAIProvider::new(api_key, &config.model)),
        };

        Self { config, provider, tools: Vec::new(), messages: Vec::new() }
    }

    pub fn add_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    pub fn get_tools(&self) -> &[Box<dyn Tool>] { &self.tools }

    pub async fn run(&mut self, input: &str) -> AgentResult {
        self.messages.push(AgentMessage { role: "system".into(), content: self.config.system_prompt.clone() });
        self.messages.push(AgentMessage { role: "user".into(), content: input.into() });

        let mut iterations = 0u32;
        let mut total_in = 0u32;
        let mut total_out = 0u32;

        while iterations < self.config.max_iterations {
            iterations += 1;

            // ---- THINK ----
            let llm_messages: Vec<LLMMessage> = self.messages.iter().map(|m| LLMMessage {
                role: m.role.clone(), content: m.content.clone(),
                tool_call_id: None, name: None,
            }).collect();

            let llm_tools = self.tools.iter().map(|t| crate::llm::LLMTool {
                name: t.name().into(), description: t.description().into(),
                parameters: t.parameters(),
            }).collect::<Vec<_>>();

            match self.provider.complete(&llm_messages, &llm_tools).await {
                Ok(response) => {
                    total_in += response.tokens_in;
                    total_out += response.tokens_out;

                    if let Some(content) = &response.content {
                        self.messages.push(AgentMessage { role: "assistant".into(), content: content.clone() });

                        // ---- ACT ----
                        if !response.tool_calls.is_empty() {
                            for tc in &response.tool_calls {
                                if let Some(tool) = self.tools.iter().find(|t| t.name() == tc.name) {
                                    let result = tool.execute(&tc.arguments).await;
                                    self.messages.push(AgentMessage {
                                        role: "tool".into(),
                                        content: if result.success { result.output } else { format!("Error: {}", result.error.unwrap_or_default()) },
                                    });
                                }
                            }
                        } else {
                            // No tool calls -> final answer
                            return AgentResult {
                                success: true,
                                output: content.clone(),
                                iterations,
                                tokens_in: total_in,
                                tokens_out: total_out,
                                error: None,
                            };
                        }
                    }
                }
                Err(e) => {
                    return AgentResult {
                        success: false, output: String::new(), iterations,
                        tokens_in: total_in, tokens_out: total_out,
                        error: Some(e.to_string()),
                    };
                }
            }

            // ---- OBSERVE: compress if too long ----
            let estimated: usize = self.messages.iter().map(|m| m.content.len()).sum();
            if estimated > (self.config.max_tokens as usize) * 3 {
                let system = self.messages.first().cloned();
                let recent = self.messages.iter().rev().take(6).cloned().collect::<Vec<_>>();
                self.messages.clear();
                if let Some(s) = system { self.messages.push(s); }
                self.messages.extend(recent.into_iter().rev());
            }
        }

        AgentResult {
            success: false, output: "Max iterations reached".into(), iterations,
            tokens_in: total_in, tokens_out: total_out,
            error: Some("Exceeded max_iterations".into()),
        }
    }

    pub fn reset(&mut self) { self.messages.clear(); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_config_default() {
        let config = AgentConfig::default();
        assert_eq!(config.model, "gpt-4o");
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.temperature, 0.7);
    }

    #[test]
    fn test_agent_result_serialization() {
        let result = AgentResult {
            success: true,
            output: "test".into(),
            iterations: 3,
            tokens_in: 100,
            tokens_out: 50,
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("true"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_event_queue_emit_poll() {
        use crate::event_queue::{emit, poll, MemFlowEvent};
        emit(MemFlowEvent::TaskCompleted {
            task_id: "t1".into(),
            workflow_id: "w1".into(),
            duration_ms: 100,
            success: true,
        });
        let event = poll();
        assert!(event.is_some());
    }
}
