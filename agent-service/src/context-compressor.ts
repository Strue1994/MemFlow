/**
 * Context Compressor — DeerFlow/Codex-aligned three-layer context management
 *
 * Layer 1 (Tool Output Budget): Externalize large tool outputs to disk
 * Layer 2 (Microcompact): Compress old messages into summaries
 * Layer 3 (Full Compact): LLM-assisted conversation compression
 */

import * as fs from "node:fs";
import * as path from "node:path";

// ---- Types ----

export interface CompactResult {
  layer: "budget" | "micro" | "full";
  originalTokens: number;
  compressedTokens: number;
  savings: number;
  detail: string;
}

interface ToolOutputExternalized {
  index: number;
  originalLength: number;
  summaryLength: number;
  storagePath: string;
}

// ---- Config ----

const TOOL_BUDGET_THRESHOLD = 2000;    // tokens
const MICROCOMPACT_THRESHOLD = 8000;   // tokens
const FULL_COMPACT_THRESHOLD = 15000;  // tokens

// Rough token estimation (4 chars ≈ 1 token)
function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}

function getStorageDir(): string {
  const root = process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(process.cwd(), "..", ".memflow-runtime");
  const dir = path.resolve(root, "context-store");
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  return dir;
}

// ---- Layer 1: Tool Output Budget ----

/**
 * Externalize oversized tool outputs to storage files.
 * Returns modified messages array with shortened content + metadata.
 */
export function applyToolBudget(messages: any[]): { messages: any[]; externalized: ToolOutputExternalized[] } {
  const externalized: ToolOutputExternalized[] = [];
  const modified = messages.map((msg, i) => {
    if (msg.role !== "tool" || !msg.content) return msg;

    const tokenEstimate = estimateTokens(String(msg.content));
    if (tokenEstimate <= TOOL_BUDGET_THRESHOLD) return msg;

    // Externalize to storage
    const storageDir = getStorageDir();
    const fileName = `tool_output_${Date.now()}_${i}.json`;
    const storagePath = path.join(storageDir, fileName);

    try {
      fs.writeFileSync(storagePath, JSON.stringify({
        original: String(msg.content),
        capturedAt: new Date().toISOString(),
        toolCallId: msg.tool_call_id,
      }), "utf-8");

      const summary = String(msg.content).slice(0, 300) + "...";
      externalized.push({
        index: i,
        originalLength: String(msg.content).length,
        summaryLength: summary.length,
        storagePath,
      });

      return {
        ...msg,
        content: `[Tool output externalized (${String(msg.content).length} chars → ${summary.length} chars). Full output at: ${storagePath}]\n${summary}`,
      };
    } catch {
      return msg; // If storage fails, keep original
    }
  });

  return { messages: modified, externalized };
}

// ---- Layer 2: Microcompact ----

/**
 * Compress old conversation rounds into a single summary message.
 * Keeps: system prompt + last 2 user/assistant exchanges + all tool results for last 2 exchanges.
 */
export function microCompact(messages: any[]): { messages: any[]; summary: string } {
  if (messages.length <= 4) return { messages, summary: "" };

  // Find system prompt
  const systemIdx = messages.findIndex((m) => m.role === "system");
  const system = systemIdx >= 0 ? [messages[systemIdx]] : [];

  // Keep last 4 messages (2 exchanges)
  const recent = messages.slice(-4);

  // Summarize the middle part
  const middle = systemIdx >= 0
    ? messages.slice(systemIdx + 1, -4)
    : messages.slice(0, -4);

  if (middle.length === 0) return { messages, summary: "" };

  // Create a compressed summary of middle messages
  const summaryParts: string[] = [];
  let userCount = 0;
  let assistantCount = 0;
  let toolCount = 0;

  for (const m of middle) {
    if (m.role === "user") userCount++;
    else if (m.role === "assistant") {
      assistantCount++;
      // Extract key points from assistant responses
      const content = String(m.content || "");
      if (content.length > 0) {
        const firstLine = content.split("\n")[0].slice(0, 150);
        summaryParts.push(`Agent: ${firstLine}`);
      }
    } else if (m.role === "tool") toolCount++;
  }

  const summary = `[Compact: ${userCount} user messages, ${assistantCount} agent responses, ${toolCount} tool calls compressed]\n${summaryParts.slice(-3).join("\n")}`;

  return {
    messages: [...system, { role: "system", content: `${system[0]?.content || ""}\n\n${summary}` }, ...recent],
    summary,
  };
}

// ---- Layer 3: Full Compact ----

/**
 * Full conversation compression using the LLM.
 * When no LLM is available, falls back to aggressive micro-compact.
 */
export async function fullCompact(
  messages: any[],
  llmCompactFn?: (history: string) => Promise<string>,
): Promise<{ messages: any[]; summary: string }> {
  if (messages.length <= 2) return { messages, summary: "" };

  // Find system prompt
  const systemIdx = messages.findIndex((m) => m.role === "system");
  const system = systemIdx >= 0 ? messages[systemIdx].content : "";

  // Keep system + last 2 exchanges + their tool results
  const keepCount = 6; // system + last 2 user + last 2 assistant + some tools
  const keepStart = Math.max(0, messages.length - keepCount);
  const toCompact = systemIdx >= 0
    ? messages.slice(systemIdx + 1, keepStart)
    : messages.slice(0, keepStart);

  if (toCompact.length === 0) return { messages, summary: "" };

  // Build compressed summary
  let summary = "";
  if (llmCompactFn) {
    try {
      const historyText = toCompact
        .map((m) => `[${m.role}]: ${String(m.content || "").slice(0, 500)}`)
        .join("\n");
      summary = await llmCompactFn(historyText);
    } catch {
      summary = buildFallbackSummary(toCompact);
    }
  } else {
    summary = buildFallbackSummary(toCompact);
  }

  const recent = messages.slice(keepStart);
  const compactedSystem = `${system}\n\n[Context Summary]\n${summary}`;

  return {
    messages: [{ role: "system", content: compactedSystem }, ...recent],
    summary,
  };
}

function buildFallbackSummary(toCompact: any[]): string {
  let userMsgs = 0;
  let assistantMsgs = 0;
  let toolCalls = 0;
  const topics = new Set<string>();

  for (const m of toCompact) {
    if (m.role === "user") userMsgs++;
    else if (m.role === "assistant") assistantMsgs++;
    else if (m.role === "tool") toolCalls++;

    // Extract potential topics
    const content = String(m.content || "");
    const words = content.split(/\s+/).filter((w) => w.length > 6);
    words.slice(0, 3).forEach((w) => topics.add(w.toLowerCase()));
  }

  const topicList = [...topics].slice(0, 8).join(", ");
  return `Conversation summary: ${userMsgs} user messages, ${assistantMsgs} agent responses, ${toolCalls} tool calls. Key topics: ${topicList}.`;
}

// ---- Orchestrator ----

/**
 * Run the full compression pipeline: budget → micro → full.
 * Each layer only activates if the previous one isn't sufficient.
 */
export async function compressContext(
  messages: any[],
  options?: { llmCompactFn?: (history: string) => Promise<string> },
): Promise<{ messages: any[]; results: CompactResult[] }> {
  const results: CompactResult[] = [];
  let current = [...messages];

  // Phase 1: Tool Output Budget
  const budgetResult = applyToolBudget(current);
  if (budgetResult.externalized.length > 0) {
    const originalTokens = estimateTokens(JSON.stringify(current));
    const compressedTokens = estimateTokens(JSON.stringify(budgetResult.messages));
    results.push({
      layer: "budget",
      originalTokens,
      compressedTokens,
      savings: originalTokens - compressedTokens,
      detail: `Externalized ${budgetResult.externalized.length} large tool output(s)`,
    });
    current = budgetResult.messages;
  }

  // Phase 2: Microcompact (if still over threshold)
  const afterBudgetTokens = estimateTokens(JSON.stringify(current));
  if (afterBudgetTokens > MICROCOMPACT_THRESHOLD) {
    const microResult = microCompact(current);
    if (microResult.summary) {
      const compressedTokens = estimateTokens(JSON.stringify(microResult.messages));
      results.push({
        layer: "micro",
        originalTokens: afterBudgetTokens,
        compressedTokens,
        savings: afterBudgetTokens - compressedTokens,
        detail: `Compressed ${current.length - microResult.messages.length} messages into summary`,
      });
      current = microResult.messages;
    }
  }

  // Phase 3: Full Compact (if still over threshold)
  const afterMicroTokens = estimateTokens(JSON.stringify(current));
  if (afterMicroTokens > FULL_COMPACT_THRESHOLD) {
    const fullResult = await fullCompact(current, options?.llmCompactFn);
    if (fullResult.summary) {
      const compressedTokens = estimateTokens(JSON.stringify(fullResult.messages));
      results.push({
        layer: "full",
        originalTokens: afterMicroTokens,
        compressedTokens,
        savings: afterMicroTokens - compressedTokens,
        detail: `Full conversation compaction applied (saved ${afterMicroTokens - compressedTokens} tokens)`,
      });
      current = fullResult.messages;
    }
  }

  return { messages: current, results };
}
