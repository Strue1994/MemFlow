/**
 * Agent Configuration — externalize agent behavior into config files
 *
 * Each agent is defined by a config with:
 *   - system_prompt: Base system prompt
 *   - tools: Allowed tool list
 *   - model: Model preference
 *   - max_iterations: Loop limit
 *   - error_handling: Retry/fallback policy
 *
 * Configs are stored in .memflow-runtime/agents/*.json
 */

import * as fs from "node:fs";
import * as path from "node:path";

// ---- Types ----

export interface AgentConfig {
  id: string;
  name: string;
  description: string;
  systemPrompt: string;
  tools: string[];
  model: string;
  provider: string;
  maxIterations: number;
  temperature: number;
  thinking: boolean;
  errorHandling: {
    retryOnFailure: boolean;
    maxRetries: number;
    fallbackModel: string;
  };
  memory: {
    enabled: boolean;
    maxContextMessages: number;
  };
}

// ---- Default ----

export const DEFAULT_AGENT_CONFIG: AgentConfig = {
  id: "default",
  name: "Default MemFlow Agent",
  description: "General-purpose agent with full tool access",
  systemPrompt: `You are MemFlow Agent — an intelligent workflow automation platform.
Your core capabilities include executing workflows, searching memory, and creating reusable skills.
Choose the right tool and complete the task efficiently.`,
  tools: ["execute_workflow", "list_workflows", "search_memory", "store_memory", "list_skills"],
  model: "gpt-4o-mini",
  provider: "auto",
  maxIterations: 10,
  temperature: 0.7,
  thinking: false,
  errorHandling: {
    retryOnFailure: true,
    maxRetries: 2,
    fallbackModel: "gpt-4o",
  },
  memory: {
    enabled: true,
    maxContextMessages: 50,
  },
};

// ---- Paths ----

function getAgentsDir(): string {
  const root = process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(process.cwd(), "..", ".memflow-runtime");
  return path.resolve(root, "agents");
}

function getAgentPath(id: string): string {
  return path.resolve(getAgentsDir(), `${id}.json`);
}

// ---- CRUD ----

export function listAgentConfigs(): AgentConfig[] {
  const dir = getAgentsDir();
  if (!fs.existsSync(dir)) return [DEFAULT_AGENT_CONFIG];

  const files = fs.readdirSync(dir).filter((f) => f.endsWith(".json"));
  if (files.length === 0) return [DEFAULT_AGENT_CONFIG];

  return files.map((f) => {
    try {
      return JSON.parse(fs.readFileSync(path.join(dir, f), "utf-8")) as AgentConfig;
    } catch {
      return null;
    }
  }).filter(Boolean) as AgentConfig[];
}

export function getAgentConfig(id: string): AgentConfig | null {
  if (id === "default") return DEFAULT_AGENT_CONFIG;

  const filePath = getAgentPath(id);
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf-8")) as AgentConfig;
  } catch {
    return null;
  }
}

export function saveAgentConfig(config: AgentConfig): void {
  const dir = getAgentsDir();
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(getAgentPath(config.id), JSON.stringify(config, null, 2), "utf-8");
}

export function deleteAgentConfig(id: string): boolean {
  try {
    fs.unlinkSync(getAgentPath(id));
    return true;
  } catch {
    return false;
  }
}

export function getAgentPrompt(id: string, taskText?: string): string {
  const config = getAgentConfig(id) || DEFAULT_AGENT_CONFIG;
  let prompt = config.systemPrompt;

  if (taskText) {
    prompt += `\n\n[Task]\n${taskText}`;
  }

  return prompt;
}
