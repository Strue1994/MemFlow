import { z } from "zod";

export const MemFlowConfig = z.object({
  baseUrl: z.string().default("http://localhost:3300"),
  apiKey: z.string().optional(),
});
export type MemFlowConfig = z.infer<typeof MemFlowConfig>;

export class MemFlow {
  private config: MemFlowConfig;

  constructor(config: Partial<MemFlowConfig> = {}) {
    this.config = MemFlowConfig.parse(config);
  }

  private get headers(): Record<string, string> {
    const h: Record<string, string> = { "Content-Type": "application/json" };
    if (this.config.apiKey) h["X-API-Key"] = this.config.apiKey;
    return h;
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const resp = await fetch(`${this.config.baseUrl}${path}`, {
      method, headers: this.headers,
      body: body ? JSON.stringify(body) : undefined,
    });
    if (!resp.ok) throw new Error(`MemFlow API error: ${resp.status} ${await resp.text()}`);
    return resp.json();
  }

  workflows = {
    execute: (id: string, params?: Record<string, unknown>) =>
      this.request("POST", "/execute", { workflow_id: id, params }),
    list: () => this.request<{ workflows: Array<{ id: string }> }>("GET", "/workflows"),
  };

  memory = {
    search: (query: string, k = 5) =>
      this.request(`GET`, `/memories/search?q=${encodeURIComponent(query)}&k=${k}`),
    store: (content: string, type = "Conversation", importance = 0.5) =>
      this.request("POST", "/memories", { content, type, importance }),
  };

  skills = {
    list: () => this.request<{ skills: Array<{ name: string }> }>("GET", "/skills"),
    create: (name: string, description: string, pattern?: string) =>
      this.request("POST", "/skills", { name, description, pattern }),
  };

  agent = {
    execute: (text: string) =>
      this.request<{ success: boolean; output: string }>("POST", "/agent/execute", { text }),
  };
}
