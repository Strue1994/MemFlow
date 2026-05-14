/**
 * T1.1: Real LLM Agent Loop
 * 
 * Replaces the Python mock_llm_think() with a proper Think-Act-Observe
 * loop that supports multiple LLM providers, tool calling, and error recovery.
 */

import OpenAI from 'openai';
import { getLLMSettings } from './llm_settings';

// ---- Types ----

export interface AgentConfig {
  model: string;
  provider: string;
  systemPrompt: string;
  maxIterations: number;
  maxTokens: number;
  temperature: number;
  tools: Tool[];
}

export interface Tool {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
  execute: (args: Record<string, unknown>) => Promise<string>;
}

export interface AgentMessage {
  role: 'system' | 'user' | 'assistant' | 'tool';
  content: string;
  tool_call_id?: string;
  name?: string;
}

export interface AgentResult {
  success: boolean;
  output: string;
  iterations: number;
  tokensUsed: { input: number; output: number };
  error?: string;
}

// ---- Agent Loop Engine ----

export class AgentLoop {
  private config: AgentConfig;
  private client: OpenAI;
  private messages: AgentMessage[] = [];

  constructor(config: Partial<AgentConfig> = {}) {
    this.config = {
      model: 'gpt-4o',
      provider: 'openai',
      systemPrompt: 'You are a helpful AI agent. Use the available tools to complete the user\'s task.',
      maxIterations: 10,
      maxTokens: 4096,
      temperature: 0.7,
      tools: [],
      ...config,
    };
    this.client = new OpenAI({
      apiKey: process.env.OPENAI_API_KEY || '',
      baseURL: process.env.OPENAI_BASE_URL || undefined,
    });
  }

  addTool(tool: Tool): void {
    this.config.tools.push(tool);
  }

  getTools(): Tool[] { return this.config.tools; }

  async run(userInput: string): Promise<AgentResult> {
    this.messages = [
      { role: 'system', content: this.config.systemPrompt },
      { role: 'user', content: userInput },
    ];

    let iterations = 0;
    let totalInputTokens = 0;
    let totalOutputTokens = 0;

    while (iterations < this.config.maxIterations) {
      iterations++;

      // ---- THINK ----
      const openAITools = this.config.tools.map((t) => ({
        type: 'function' as const,
        function: {
          name: t.name,
          description: t.description,
          parameters: t.parameters as Record<string, unknown>,
        },
      }));

      try {
        const completion = await this.client.chat.completions.create({
          model: this.config.model,
          messages: this.messages as any,
          tools: openAITools.length > 0 ? openAITools : undefined,
          tool_choice: openAITools.length > 0 ? 'auto' : undefined,
          max_tokens: this.config.maxTokens,
          temperature: this.config.temperature,
        });

        const choice = completion.choices[0];
        const message = choice.message;

        if (message.content) {
          this.messages.push({ role: 'assistant', content: message.content });
        }

        totalInputTokens += completion.usage?.prompt_tokens || 0;
        totalOutputTokens += completion.usage?.completion_tokens || 0;

        // ---- ACT (if tool calls) ----
        if (message.tool_calls && message.tool_calls.length > 0) {
          for (const toolCall of message.tool_calls) {
            const tool = this.config.tools.find((t) => t.name === toolCall.function.name);
            if (!tool) {
              this.messages.push({
                role: 'tool',
                content: `Error: Tool "${toolCall.function.name}" not found`,
                tool_call_id: toolCall.id,
                name: toolCall.function.name,
              });
              continue;
            }

            let result: string;
            try {
              const args = JSON.parse(toolCall.function.arguments);
              result = await tool.execute(args);
            } catch (err: any) {
              result = `Error executing tool: ${err.message}`;
            }

            this.messages.push({
              role: 'tool',
              content: result,
              tool_call_id: toolCall.id,
              name: toolCall.function.name,
            });
          }
        } else {
          // No tool calls -> final answer
          return {
            success: true,
            output: message.content || '',
            iterations,
            tokensUsed: { input: totalInputTokens, output: totalOutputTokens },
          };
        }

        // ---- OBSERVE: compress context if too long ----
        if (this.estimateTokens() > this.config.maxTokens * 0.8) {
          this.compressContext();
        }

      } catch (err: any) {
        return {
          success: false,
          output: '',
          iterations,
          tokensUsed: { input: totalInputTokens, output: totalOutputTokens },
          error: `LLM call failed: ${err.message}`,
        };
      }
    }

    return {
      success: false,
      output: 'Max iterations reached without completing task',
      iterations,
      tokensUsed: { input: totalInputTokens, output: totalOutputTokens },
      error: 'Exceeded maxIterations',
    };
  }

  private estimateTokens(): number {
    return this.messages.reduce((sum, m) => sum + m.content.length / 4, 0);
  }

  private compressContext(): void {
    // Keep system message + last N messages
    const systemMsg = this.messages.find((m) => m.role === 'system');
    const recentMessages = this.messages.slice(-6);
    this.messages = systemMsg ? [systemMsg, ...recentMessages] : recentMessages;
  }

  reset(): void {
    this.messages = [];
  }
}

// ---- Factory ----

export async function createAgentLoop(tools?: Tool[]): Promise<AgentLoop> {
  const settings = await getLLMSettings();
  const loop = new AgentLoop({
    model: settings.model || 'gpt-4o',
    provider: settings.provider || 'openai',
    systemPrompt: `You are MemFlow Agent — an intelligent workflow automation agent.
You have access to various tools for executing workflows, managing memory, and performing tasks.
Choose the right tool and complete the user's request efficiently.`,
  });

  if (tools) {
    for (const tool of tools) {
      loop.addTool(tool);
    }
  }

  return loop;
}

export default AgentLoop;
