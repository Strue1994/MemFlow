/**
 * MemFlow TypeScript SDK — Full API client
 *
 * Covers: agent, router, MCP, curator, checkpoints, sandbox,
 * subagents, skills, providers, channels, and setup.
 *
 * Usage:
 *   import { MemFlow } from "@memflow/sdk";
 *   const mf = new MemFlow({ baseUrl: "http://localhost:3000" });
 *   const result = await mf.agent.execute("hello");
 */

// ---- Types ----

export interface MemFlowOptions {
  baseUrl?: string;
  apiKey?: string;
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

export interface RouterConfig {
  mode: "auto" | "manual";
  manualTier: string;
  escalation: boolean;
}

export interface RouterStats {
  totalCost: number;
  totalCalls: number;
  byProvider: Record<string, { calls: number; cost: number }>;
}

export interface MCPServerConfig {
  name: string;
  transport: "stdio" | "sse";
  command?: string;
  args?: string[];
  url?: string;
}

export interface CuratorReport {
  cycleId: string;
  newSkills: number;
  mergedSkills: number;
  prunedSkills: number;
  totalSkills: number;
  summary: string;
}

export interface Checkpoint {
  id: string;
  sessionId: string;
  iteration: number;
  messageCount: number;
  messages: any[];
  summary: string;
}

export interface Skill {
  id: string;
  name: string;
  description: string;
  category: string;
}

export interface ProviderEntry {
  id: string;
  label: string;
  enabled: boolean;
  apiKey?: string;
  model: string;
  tier: string;
}

export interface ChannelEntry {
  id: string;
  label: string;
  enabled: boolean;
  config: Record<string, string>;
}

export interface MiddlewareEntry {
  name: string;
  description: string;
  enabled: boolean;
  priority: number;
}

export interface SetupStatus {
  needsSetup: boolean;
  hasProviders: boolean;
  hasChannels: boolean;
  providers: string[];
  channels: string[];
  providerTemplates: any[];
  channelTemplates: any[];
}

// ---- Client ----

export class MemFlow {
  private baseUrl: string;
  private apiKey?: string;

  constructor(options: MemFlowOptions = {}) {
    this.baseUrl = options.baseUrl || "http://localhost:3000";
    this.apiKey = options.apiKey;
  }

  private get headers(): Record<string, string> {
    const h: Record<string, string> = { "Content-Type": "application/json" };
    if (this.apiKey) h["X-API-Key"] = this.apiKey;
    return h;
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const resp = await fetch(url, {
      method,
      headers: this.headers,
      body: body ? JSON.stringify(body) : undefined,
    });
    if (!resp.ok) {
      const text = await resp.text().catch(() => resp.statusText);
      throw new Error(`MemFlow ${method} ${path}: ${resp.status} ${text.slice(0, 200)}`);
    }
    return resp.json() as Promise<T>;
  }

  // ======== Agent ========

  agent = {
    execute: (text: string, options?: { stream?: boolean; tools?: boolean }) =>
      this.request<AgentResult>("POST", "/agent/execute", { text, stream: options?.stream }),

    chat: (messages: { role: string; content: string }[], options?: { model?: string; stream?: boolean }) =>
      this.request<any>("POST", "/v1/chat/completions", {
        model: options?.model || "gpt-4o",
        messages,
        stream: options?.stream || false,
      }),
  };

  // ======== Router ========

  router = {
    config: {
      get: () => this.request<{ config: RouterConfig; providers: string[] }>("GET", "/router/config"),
      set: (cfg: Partial<RouterConfig>) => this.request<{ config: RouterConfig }>("POST", "/router/config", cfg),
    },
    stats: () => this.request<RouterStats>("GET", "/router/stats"),
  };

  // ======== MCP ========

  mcp = {
    list: () => this.request<{ configs: MCPServerConfig[]; connections: any[] }>("GET", "/mcp/servers"),
    register: (config: MCPServerConfig) => this.request<any>("POST", "/mcp/servers", config),
    unregister: (name: string) => this.request<{ deleted: boolean }>("DELETE", `/mcp/servers/${name}`),
    connect: (name: string) => this.request<any>("POST", `/mcp/servers/${name}/connect`),
    disconnect: (name: string) => this.request<any>("POST", `/mcp/servers/${name}/disconnect`),
  };

  // ======== Curator ========

  curator = {
    run: () => this.request<{ report: CuratorReport }>("POST", "/curator/run"),
    status: () => this.request<any>("GET", "/curator/status"),
    history: () => this.request<any>("GET", "/curator/history"),
  };

  // ======== Checkpoints ========

  checkpoints = {
    list: () => this.request<{ checkpoints: any[]; stats: any }>("GET", "/checkpoints"),
    latest: (sessionId?: string) =>
      this.request<{ checkpoint: Checkpoint }>("GET", `/checkpoints/latest${sessionId ? `?sessionId=${sessionId}` : ""}`),
    save: (sessionId: string, messages: any[]) =>
      this.request<{ checkpoint: Checkpoint }>("POST", "/checkpoints/save", { sessionId, messages }),
    delete: (sessionId: string) =>
      this.request<{ deleted: boolean }>("DELETE", `/checkpoints/${sessionId}`),
  };

  // ======== Skills ========

  skills = {
    list: () => this.request<{ skills: Skill[] }>("GET", "/skills"),
    import: (dir?: string) => this.request<{ imported: number; skills: Skill[] }>("POST", "/skills/import", { dir }),
    imported: () => this.request<{ skills: Skill[]; count: number }>("GET", "/skills/imported"),
  };

  // ======== Middleware ========

  middleware = {
    list: () => this.request<{ middlewares: MiddlewareEntry[] }>("GET", "/middleware/config"),
    toggle: (name: string, enabled: boolean) =>
      this.request<{ name: string; enabled: boolean }>("POST", `/middleware/config/${name}/toggle`, { enabled }),
  };

  // ======== Providers ========

  providers = {
    list: () => this.request<{ providers: ProviderEntry[] }>("GET", "/providers"),
    add: (id: string, config: Partial<ProviderEntry>) =>
      this.request<{ provider: ProviderEntry }>("POST", "/providers", { id, ...config }),
    remove: (id: string) => this.request<{ deleted: boolean }>("DELETE", `/providers/${id}`),
  };

  // ======== Channels ========

  channels = {
    list: () => this.request<{ channels: ChannelEntry[] }>("GET", "/channels"),
    add: (id: string, config: Record<string, string>, enabled = false) =>
      this.request<{ channel: ChannelEntry }>("POST", "/channels", { id, config, enabled }),
    remove: (id: string) => this.request<{ deleted: boolean }>("DELETE", `/channels/${id}`),
  };

  // ======== Setup ========

  setup = {
    status: () => this.request<SetupStatus>("GET", "/setup/status"),
  };

  // ======== Context ========

  compact = (messages: any[]) =>
    this.request<{ messages: any[]; results: any[]; savings: number }>("POST", "/compact", { messages });

  // ======== Sandbox ========

  sandbox = {
    config: {
      get: () => this.request<any>("GET", "/sandbox/config"),
      set: (cfg: any) => this.request<any>("POST", "/sandbox/config", cfg),
    },
    execute: (command: string, args: string[]) =>
      this.request<any>("POST", "/sandbox/execute", { command, args }),
  };

  // ======== Subagents ========

  subagents = {
    list: () => this.request<any>("GET", "/subagents"),
    spawn: (spec: { name: string; prompt: string; timeout_secs?: number }) =>
      this.request<any>("POST", "/subagents/spawn", spec),
    status: (id: string) => this.request<any>("GET", `/subagents/${id}/status`),
    cancel: (id: string) => this.request<any>("POST", `/subagents/${id}/cancel`),
  };
}

// ---- Default export ----

export default MemFlow;
