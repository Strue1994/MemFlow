/**
 * Agent Loop: Aligned with Hermes/OpenClaw capabilities
 * 
 * Hermes-aligned features:
 * - Multi-provider (OpenAI, Anthropic, Gemini, Groq, DeepSeek, Together, Perplexity, xAI, Mistral)
 * - Smart routing by task complexity (simple → medium → complex → expert)
 * - Provider fallback chain with cost tracking
 * - Thinking mode for complex tasks
 * - System prompt assembly from memory + skills
 * - Streaming support (OpenAI, Anthropic)
 * - Tool calling with error recovery
 */

import { getLLMSettings } from "./llm_settings";
import { SkillManager } from "./skill-system";
import { selectProviders, globalCostTracker, getRouterConfig, type RouterProvider } from "./llm_router";
import { discoverAllSkills } from "./skill-loader";
import { globalCurator } from "./curator";

// ---- Types ----

export interface Tool {
  name: string;
  description: string;
  parameters: Record<string, unknown>;
  execute: (args: Record<string, unknown>) => Promise<string>;
}

export interface AgentResult {
  success: boolean;
  output: string;
  iterations: number;
  tokensUsed: { input: number; output: number };
  model: string;
  error?: string;
  tier?: string;
  cost?: number;
}

interface ProviderConfig {
  name: string;
  baseUrl: string;
  apiKey: string;
  model: string;
}

interface ProviderClient {
  readonly id: string;
  readonly modelName: string;
  complete(messages: any[], tools?: any[], thinking?: boolean): Promise<ProviderResponse>;
  stream?(messages: any[], tools?: any[]): AsyncIterable<string>;
}

interface ProviderResponse {
  content: string | null;
  toolCalls: Array<{ id: string; name: string; arguments: string }>;
  tokensIn: number;
  tokensOut: number;
  model: string;
}

// ---- OpenAI Provider (also handles Groq, DeepSeek, Together, Perplexity, xAI, OpenRouter, Ollama) ----

class OpenAIProvider implements ProviderClient {
  private cfg: ProviderConfig;
  readonly id: string;
  readonly modelName: string;

  constructor(cfg: ProviderConfig, id?: string) {
    this.cfg = cfg;
    this.id = id || cfg.name;
    this.modelName = cfg.model;
  }

  async complete(messages: any[], tools?: any[], thinking?: boolean): Promise<ProviderResponse> {
    const body: any = {
      model: this.cfg.model,
      messages,
      max_tokens: 4096,
    };
    if (tools && tools.length > 0) {
      body.tools = tools.map((t) => ({
        type: "function",
        function: { name: t.name, description: t.description, parameters: t.parameters },
      }));
      body.tool_choice = "auto";
    }

    const resp = await fetch(`${this.cfg.baseUrl}/chat/completions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Authorization": `Bearer ${this.cfg.apiKey}`,
      },
      body: JSON.stringify(body),
    });

    if (!resp.ok) {
      const err = await resp.text();
      throw new Error(`${resp.status} ${err.slice(0, 200)}`);
    }

    const data = await resp.json();
    const choice = data.choices?.[0]?.message;

    return {
      content: choice?.content ?? null,
      toolCalls: (choice?.tool_calls || []).map((tc: any) => ({
        id: tc.id,
        name: tc.function.name,
        arguments: tc.function.arguments,
      })),
      tokensIn: data.usage?.prompt_tokens ?? 0,
      tokensOut: data.usage?.completion_tokens ?? 0,
      model: data.model || this.cfg.model,
    };
  }

  async *stream(messages: any[], tools?: any[]): AsyncIterable<string> {
    const body: any = { model: this.cfg.model, messages, stream: true };
    if (tools && tools.length > 0) { body.tools = tools; }

    const resp = await fetch(`${this.cfg.baseUrl}/chat/completions`, {
      method: "POST",
      headers: { "Content-Type": "application/json", "Authorization": `Bearer ${this.cfg.apiKey}` },
      body: JSON.stringify(body),
    });

    if (!resp.ok) throw new Error(`Stream error: ${resp.status}`);
    const reader = resp.body!.getReader();
    const decoder = new TextDecoder();

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      const chunk = decoder.decode(value);
      for (const line of chunk.split("\n")) {
        if (line.startsWith("data: ") && line !== "data: [DONE]") {
          try {
            const json = JSON.parse(line.slice(6));
            yield json.choices?.[0]?.delta?.content || "";
          } catch { /* skip parse errors */ }
        }
      }
    }
  }
}

// ---- Anthropic Provider ----

class AnthropicProvider implements ProviderClient {
  private cfg: ProviderConfig;
  readonly id: string;
  readonly modelName: string;

  constructor(cfg: ProviderConfig, id?: string) {
    this.cfg = cfg;
    this.id = id || cfg.name;
    this.modelName = cfg.model;
  }

  async complete(messages: any[], tools?: any[], thinking?: boolean): Promise<ProviderResponse> {
    const systemMsg = messages.find((m: any) => m.role === "system");
    const body: any = {
      model: this.cfg.model,
      max_tokens: 4096,
      messages: messages.filter((m: any) => m.role !== "system"),
    };
    if (systemMsg) body.system = systemMsg.content;
    if (tools && tools.length > 0) {
      body.tools = tools.map((t: any) => ({
        name: t.name, description: t.description, input_schema: t.parameters,
      }));
    }

    const resp = await fetch("https://api.anthropic.com/v1/messages", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "x-api-key": this.cfg.apiKey,
        "anthropic-version": "2023-06-01",
      },
      body: JSON.stringify(body),
    });

    if (!resp.ok) { const err = await resp.text(); throw new Error(`${resp.status} ${err.slice(0, 200)}`); }
    const data = await resp.json();

    // Parse content blocks: text + tool_use
    let content: string | null = null;
    const toolCalls: Array<{ id: string; name: string; arguments: string }> = [];
    for (const block of data.content || []) {
      if (block.type === "text") content = (content || "") + block.text;
      if (block.type === "tool_use") {
        toolCalls.push({ id: block.id, name: block.name, arguments: JSON.stringify(block.input) });
      }
    }

    return {
      content,
      toolCalls,
      tokensIn: data.usage?.input_tokens ?? 0,
      tokensOut: data.usage?.output_tokens ?? 0,
      model: data.model || this.cfg.model,
    };
  }

  async *stream(messages: any[], tools?: any[]): AsyncIterable<string> {
    const systemMsg = messages.find((m: any) => m.role === "system");
    const body: any = {
      model: this.cfg.model, max_tokens: 4096, stream: true,
      messages: messages.filter((m: any) => m.role !== "system"),
    };
    if (systemMsg) body.system = systemMsg.content;

    const resp = await fetch("https://api.anthropic.com/v1/messages", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "x-api-key": this.cfg.apiKey,
        "anthropic-version": "2023-06-01",
      },
      body: JSON.stringify(body),
    });

    if (!resp.ok) throw new Error(`Anthropic stream error: ${resp.status}`);
    const reader = resp.body!.getReader();
    const decoder = new TextDecoder();
    let buffer = "";

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      for (const line of buffer.split("\n")) {
        if (line.startsWith("data: ")) {
          try {
            const json = JSON.parse(line.slice(6));
            if (json.type === "content_block_delta" && json.delta?.text) {
              yield json.delta.text;
            }
          } catch { /* skip */ }
        }
      }
      buffer = "";
    }
  }
}

// ---- Gemini Provider ----

class GeminiProvider implements ProviderClient {
  private cfg: ProviderConfig;
  readonly id: string;
  readonly modelName: string;

  constructor(cfg: ProviderConfig, id?: string) {
    this.cfg = cfg;
    this.id = id || cfg.name;
    this.modelName = cfg.model;
  }

  async complete(messages: any[], tools?: any[]): Promise<ProviderResponse> {
    // Map OpenAI-format messages to Gemini format
    const contents: any[] = [];
    let systemInstruction = "";

    for (const m of messages) {
      if (m.role === "system") {
        systemInstruction = (systemInstruction + "\n" + m.content).trim();
      } else if (m.role === "user") {
        contents.push({ role: "user", parts: [{ text: m.content }] });
      } else if (m.role === "assistant") {
        contents.push({ role: "model", parts: [{ text: m.content }] });
      }
    }

    // Gemini requires alternating user/model; merge consecutive same-role
    const merged: any[] = [];
    for (const c of contents) {
      const last = merged[merged.length - 1];
      if (last && last.role === c.role) {
        last.parts[0].text += "\n" + c.parts[0].text;
      } else {
        merged.push(c);
      }
    }

    const body: any = { contents: merged };
    if (systemInstruction) body.systemInstruction = { parts: [{ text: systemInstruction }] };

    const url = `https://generativelanguage.googleapis.com/v1beta/models/${this.cfg.model}:generateContent?key=${this.cfg.apiKey}`;
    const resp = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });

    if (!resp.ok) { const err = await resp.text(); throw new Error(`Gemini ${resp.status}: ${err.slice(0, 200)}`); }
    const data = await resp.json();

    const candidate = data.candidates?.[0];
    const text = candidate?.content?.parts?.map((p: any) => p.text).join("") || "";
    const usage = data.usageMetadata || {};

    return {
      content: text || null,
      toolCalls: [],
      tokensIn: usage.promptTokenCount ?? 0,
      tokensOut: usage.candidatesTokenCount ?? 0,
      model: this.cfg.model,
    };
  }
}

// ---- System Prompt Assembly (Hermes-aligned) ----

function assembleSystemPrompt(userId?: string, taskText?: string): string {
  const parts: string[] = [];

  // Core identity
  parts.push(`You are MemFlow Agent — an intelligent workflow automation platform.
Your core capabilities include executing workflows, searching memory, and creating reusable skills.
Choose the right tool and complete the task efficiently.`);

  // Context from memory
  if (userId) {
    parts.push(`\n[User Context]`);
    parts.push(`User ID: ${userId}`);
  }

  // Task context
  if (taskText) {
    parts.push(`\n[Task]`);
    parts.push(taskText);
  }

  // Available skills (from SkillManager + SKILL.md imports)
  try {
    const sm = new SkillManager();
    // Load external SKILL.md skills from disk
    discoverAllSkills(sm);
    const skills = sm.listSkills();
    if (skills.length > 0) {
      parts.push(`\n[Available Skills]`);
      // Sort by relevance to taskText
      const sorted = taskText
        ? skills.sort((a, b) => {
            const aScore = a.keywords.filter((k) => taskText.toLowerCase().includes(k)).length;
            const bScore = b.keywords.filter((k) => taskText.toLowerCase().includes(k)).length;
            return bScore - aScore;
          })
        : skills;
      for (const s of sorted.slice(0, 8)) {
        parts.push(`- ${s.name}: ${s.description}`);
      }
    }
  } catch { /* skills dir may not exist */ }

  // Behavioral guidelines
  parts.push(`\n[Guidelines]
- Use tools when needed, reply directly when not
- If a task matches an available skill, use it
- Be concise and precise
- If you encounter errors, try alternative approaches`);

  return parts.join("\n");
}

// ---- Provider Fallback Chain (Hermes-aligned) ----

async function callWithFallback(
  providers: ProviderClient[],
  messages: any[],
  tools?: any[],
  thinking?: boolean,
): Promise<{ response: ProviderResponse; providerIndex: number }> {
  let lastError: Error | undefined;

  for (let i = 0; i < providers.length; i++) {
    try {
      const response = await providers[i].complete(messages, tools, thinking);
      return { response, providerIndex: i };
    } catch (err: any) {
      lastError = err;
      console.warn(`Provider ${i} failed: ${err.message}. Trying next...`);
    }
  }

  throw lastError || new Error("All providers failed");
}

// ---- Main Agent Loop ----

/**
 * Helper: build ProviderClient instances from RouterProvider decisions.
 */
function buildProviderClients(rp: RouterProvider[]): ProviderClient[] {
  const clients: ProviderClient[] = [];
  for (const p of rp) {
    switch (p.id) {
      case "anthropic":
        clients.push(new AnthropicProvider(
          { name: p.id, baseUrl: p.baseUrl, apiKey: p.apiKey, model: p.model },
          p.id,
        ));
        break;
      case "gemini":
        clients.push(new GeminiProvider(
          { name: p.id, baseUrl: p.baseUrl, apiKey: p.apiKey, model: p.model },
          p.id,
        ));
        break;
      default:
        // OpenAI-compatible (openai, groq, deepseek, together, perplexity, xai, mistral, openrouter, ollama)
        clients.push(new OpenAIProvider(
          { name: p.id, baseUrl: p.baseUrl, apiKey: p.apiKey, model: p.model },
          p.id,
        ));
    }
  }
  return clients;
}

export async function runAgent(
  text: string,
  options?: {
    tools?: Tool[];
    userId?: string;
    stream?: boolean;
    thinking?: boolean;
    maxIterations?: number;
  },
): Promise<AgentResult> {
  const maxIter = options?.maxIterations ?? 10;

  // Use smart router to select providers
  const toolCount = options?.tools?.length || 0;
  const decision = selectProviders(text, toolCount);

  if (decision.providers.length === 0) {
    return {
      success: false, output: "", iterations: 0, tokensUsed: { input: 0, output: 0 },
      model: "", error: "No LLM provider configured. Set any of: OPENAI_API_KEY, ANTHROPIC_API_KEY, GROQ_API_KEY, DEEPSEEK_API_KEY, GEMINI_API_KEY",
    };
  }

  const providers = buildProviderClients(decision.providers);
  const tier = decision.tier;

  const systemPrompt = assembleSystemPrompt(options?.userId, text);
  const messages: any[] = [{ role: "system", content: systemPrompt }, { role: "user", content: text }];
  const toolDefs = options?.tools?.map((t) => ({
    name: t.name, description: t.description, parameters: t.parameters,
  })) || [];

  let iterations = 0;
  let totalIn = 0;
  let totalOut = 0;
  let usedModel = providers[0]?.modelName || "unknown";

  // Streaming path
  if (options?.stream && providers[0]?.stream) {
    let fullText = "";
    try {
      for await (const chunk of providers[0].stream(messages, toolDefs.length > 0 ? toolDefs : undefined)) {
        fullText += chunk;
      }
    } catch (err: any) {
      return {
        success: false, output: fullText, iterations: 1,
        tokensUsed: { input: 0, output: 0 }, model: usedModel,
        error: `Stream error: ${err.message}`, tier, cost: 0,
      };
    }
    return {
      success: true, output: fullText, iterations: 1,
      tokensUsed: { input: 0, output: 0 }, model: usedModel, tier, cost: 0,
    };
  }

  // Think-Act-Observe loop
  let consecutiveToolOnly = 0;
  while (iterations < maxIter) {
    iterations++;

    const { response, providerIndex } = await callWithFallback(
      providers, messages, toolDefs.length > 0 ? toolDefs : undefined, options?.thinking,
    );
    totalIn += response.tokensIn;
    totalOut += response.tokensOut;
    usedModel = response.model;

    // Track cost
    globalCostTracker.record(providers[providerIndex]?.id || "unknown", usedModel, response.tokensIn, response.tokensOut);

    // [FIX] Bug 1: Empty response guard — LLM returned nothing at all
    if (!response.content && response.toolCalls.length === 0) {
      return {
        success: false, output: "", iterations,
        tokensUsed: { input: totalIn, output: totalOut }, model: usedModel,
        error: "LLM returned empty response (no content, no tool calls)", tier, cost: globalCostTracker.getTotalCost(),
      };
    }

    if (response.content) {
      messages.push({ role: "assistant", content: response.content });
    }

    // Tool calls
    if (response.toolCalls.length > 0 && options?.tools) {
      consecutiveToolOnly++;
      for (const tc of response.toolCalls) {
        const tool = options.tools.find((t) => t.name === tc.name);
        if (!tool) {
          messages.push({ role: "tool", content: `Tool "${tc.name}" not found`, tool_call_id: tc.id });
          continue;
        }
        try {
          const args = JSON.parse(tc.arguments);
          const result = await tool.execute(args);
          messages.push({ role: "tool", content: result, tool_call_id: tc.id });
        } catch (err: any) {
          messages.push({ role: "tool", content: `Error: ${err.message}`, tool_call_id: tc.id });
        }
      }
      // [FIX] Bug 2: After enough tool-only rounds, force the model to answer
      if (iterations >= Math.ceil(maxIter * 0.6)) {
        messages.push({
          role: "user",
          content: "You have executed enough tools. Based on the results above, provide a concise final answer now. Do NOT call any more tools.",
        });
      }
    } else if (response.content) {
      // Final answer — reset tool counter on success
      consecutiveToolOnly = 0;
      return {
        success: true, output: response.content, iterations,
        tokensUsed: { input: totalIn, output: totalOut }, model: usedModel,
        tier, cost: globalCostTracker.getTotalCost(),
      };
    }

    // [FIX] Bug 3: Safety valve — if every iteration was tool-only, force-exit
    if (consecutiveToolOnly >= maxIter) {
      return {
        success: false, output: messages.filter((m: any) => m.role === "assistant").pop()?.content || "",
        iterations,
        tokensUsed: { input: totalIn, output: totalOut }, model: usedModel,
        error: `Model kept calling tools for ${maxIter} iterations without producing a final answer`,
        tier, cost: globalCostTracker.getTotalCost(),
      };
    }

    // Context compression (Hermes-aligned)
    const estimatedTokens = messages.reduce((s: number, m: any) => s + (m.content?.length || 0) / 4, 0);
    if (estimatedTokens > 12000) {
      const system = messages.find((m: any) => m.role === "system");
      const last6 = messages.slice(-6);
      messages.length = 0;
      if (system) messages.push(system);
      messages.push(...last6);
    }
  }

  return {
    success: false, output: "Max iterations reached", iterations,
    tokensUsed: { input: totalIn, output: totalOut }, model: usedModel,
    error: `Exceeded ${maxIter} iterations`,
    tier, cost: globalCostTracker.getTotalCost(),
  };
}

export default runAgent;
