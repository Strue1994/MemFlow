/**
 * MCP Client — connects to MCP servers and exposes tools to the agent loop
 *
 * Supports:
 * - stdio transport (child process)
 * - SSE transport (Server-Sent Events over HTTP)
 * - Auto-discovery from config files
 * - Tool registration into the Tool interface
 */

import { spawn, type ChildProcess } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";
import * as crypto from "node:crypto";

// ---- Types ----

export interface MCPServerConfig {
  name: string;
  transport: "stdio" | "sse";
  /** For stdio: command to run */
  command?: string;
  /** For stdio: args for command */
  args?: string[];
  /** For sse: URL endpoint */
  url?: string;
  /** Environment variables */
  env?: Record<string, string>;
  /** Auto-start on service boot */
  autoStart?: boolean;
}

export interface MCPTool {
  serverName: string;
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
}

interface MCPResponse {
  jsonrpc: string;
  id: number;
  result?: unknown;
  error?: { code: number; message: string };
}

// ---- JSON-RPC helpers ----

let requestId = 1;

function createRequest(method: string, params?: unknown, id?: number): string {
  const msgId = id ?? requestId++;
  return JSON.stringify({
    jsonrpc: "2.0",
    id: msgId,
    method,
    params: params || {},
  });
}

function parseResponse(data: string): MCPResponse | null {
  try {
    return JSON.parse(data) as MCPResponse;
  } catch {
    return null;
  }
}

// ---- Transport ----

interface Transport {
  send(message: string): void;
  onMessage(callback: (msg: string) => void): void;
  onError(callback: (err: Error) => void): void;
  close(): void;
}

class StdioTransport implements Transport {
  private proc: ChildProcess;
  private buffer = "";
  private msgCallback: ((msg: string) => void) | null = null;
  private errCallback: ((err: Error) => void) | null = null;

  constructor(config: MCPServerConfig) {
    const env = { ...process.env, ...config.env } as Record<string, string>;
    const isWin = process.platform === "win32";

    let command: string;
    let args: string[];
    const spawnOpts: any = {
      stdio: ["pipe", "pipe", "pipe"],
      env,
    };

    if (isWin) {
      // Windows: must use shell:true for stdin/stdout pipes to work
      const nodePath = process.execPath;
      const scriptPath = (config.args?.[0] || "").replace(/\//g, "\\");
      const scriptArgs = (config.args?.slice(1) || []).join(" ");
      command = `"${nodePath}" "${scriptPath}" ${scriptArgs}`.trim();
      args = [];
      spawnOpts.shell = true;
      spawnOpts.windowsHide = true;
      spawnOpts.cwd = process.env.TEMP || process.cwd();
    } else {
      // Unix: use full path to node
      command = config.command || "";
      args = [...(config.args || [])];
      if (command.toLowerCase() === "node") {
        command = process.execPath;
      }
      spawnOpts.cwd = process.env.MEMFLOW_RUNTIME_ROOT || process.cwd();
    }

    this.proc = spawn(command, args, spawnOpts);

    this.proc.stdout?.on("data", (chunk: Buffer) => {
      this.buffer += chunk.toString();
      const lines = this.buffer.split("\n");
      this.buffer = lines.pop() || "";
      for (const line of lines) {
        const trimmed = line.trim();
        if (trimmed && this.msgCallback) {
          this.msgCallback(trimmed);
        }
      }
    });

    this.proc.stderr?.on("data", (chunk: Buffer) => {
      const text = chunk.toString().trim();
      if (text) {
        console.warn(`[MCP:${config.name}] ${text}`);
      }
    });

    this.proc.on("error", (err) => {
      console.warn(`[MCP:${config.name}] Process error: ${err.message}`);
      this.errCallback?.(err);
    });

    this.proc.on("exit", (code, signal) => {
      if (code !== null && code !== 0) {
        console.warn(`[MCP:${config.name}] Process exited with code ${code}`);
      }
    });
  }

  send(message: string): void {
    this.proc.stdin?.write(message + "\n");
  }

  onMessage(callback: (msg: string) => void): void {
    this.msgCallback = callback;
  }

  onError(callback: (err: Error) => void): void {
    this.errCallback = callback;
  }

  close(): void {
    this.proc.kill();
  }
}

class SSETransport implements Transport {
  private url: string;
  private abortController = new AbortController();
  private msgCallback: ((msg: string) => void) | null = null;
  private errCallback: ((err: Error) => void) | null = null;

  constructor(config: MCPServerConfig) {
    this.url = config.url || "";
  }

  async connect(): Promise<void> {
    try {
      const response = await fetch(this.url!, {
        signal: this.abortController.signal,
      });
      const reader = response.body!.getReader();
      const decoder = new TextDecoder();
      let buffer = "";

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });

        const lines = buffer.split("\n");
        buffer = lines.pop() || "";
        for (const line of lines) {
          if (line.startsWith("data: ")) {
            const data = line.slice(6).trim();
            if (data && this.msgCallback) {
              this.msgCallback(data);
            }
          }
        }
      }
    } catch (err: any) {
      if (err.name !== "AbortError") {
        this.errCallback?.(err);
      }
    }
  }

  send(message: string): void {
    // SSE transport sends via POST
    fetch(this.url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: message,
      signal: this.abortController.signal,
    }).catch((err) => this.errCallback?.(err));
  }

  onMessage(callback: (msg: string) => void): void {
    this.msgCallback = callback;
  }

  onError(callback: (err: Error) => void): void {
    this.errCallback = callback;
  }

  close(): void {
    this.abortController.abort();
  }
}

// ---- Server Connection ----

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (reason: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

export class MCPServerConnection {
  config: MCPServerConfig;
  private transport: Transport;
  private pending = new Map<number, PendingRequest>();
  private initialized = false;
  private _tools: MCPTool[] = [];

  constructor(config: MCPServerConfig) {
    this.config = config;
    this.transport = config.transport === "sse"
      ? new SSETransport(config)
      : new StdioTransport(config);

    this.transport.onMessage((msg) => this.handleMessage(msg));
    this.transport.onError((err) => {
      for (const [id, req] of this.pending) {
        clearTimeout(req.timer);
        req.reject(err);
        this.pending.delete(id);
      }
    });

    // Auto-connect SSE
    if (config.transport === "sse") {
      (this.transport as SSETransport).connect().catch(() => {});
    }
  }

  get tools(): MCPTool[] {
    return this._tools;
  }

  private handleMessage(msg: string): void {
    const response = parseResponse(msg);
    if (!response) return;

    const req = this.pending.get(response.id);
    if (!req) return;

    clearTimeout(req.timer);
    this.pending.delete(response.id);

    if (response.error) {
      req.reject(new Error(`MCP error ${response.error.code}: ${response.error.message}`));
    } else {
      req.resolve(response.result);
    }
  }

  private async request(method: string, params?: unknown, timeoutMs = 10000): Promise<unknown> {
    return new Promise((resolve, reject) => {
      const id = requestId++;
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`MCP request ${method} timed out after ${timeoutMs}ms`));
      }, timeoutMs);

      this.pending.set(id, { resolve, reject, timer });
      this.transport.send(createRequest(method, params, id));
    });
  }

  /** Initialize the MCP connection */
  async initialize(): Promise<void> {
    if (this.initialized) return;

    // Send initialize request
    const result = await this.request("initialize", {
      protocolVersion: "2025-03-26",
      capabilities: {},
      clientInfo: { name: "memflow-agent", version: "1.0.0" },
    });

    // Send initialized notification
    this.transport.send(JSON.stringify({
      jsonrpc: "2.0",
      method: "notifications/initialized",
    }));

    this.initialized = true;

    // Discover tools
    await this.refreshTools();
  }

  /** Refresh the tool list from the server */
  async refreshTools(): Promise<MCPTool[]> {
    const result = await this.request("tools/list") as { tools: any[] } | undefined;
    if (result?.tools) {
      this._tools = result.tools.map((t: any) => ({
        serverName: this.config.name,
        name: t.name,
        description: t.description || "",
        inputSchema: t.inputSchema || t.parameters || {},
      }));
    }
    return this._tools;
  }

  /** Call a tool on the server */
  async callTool(name: string, args: Record<string, unknown>): Promise<string> {
    const result = await this.request("tools/call", { name, arguments: args }) as any;
    if (result?.content) {
      return result.content.map((c: any) => c.text || JSON.stringify(c)).join("\n");
    }
    return JSON.stringify(result) || "Tool returned no output";
  }

  close(): void {
    this.transport.close();
    this.initialized = false;
  }
}

// ---- Client Manager ----

const DEFAULT_MCP_CONFIG_PATH = path.resolve(
  process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(process.cwd(), "..", ".memflow-runtime"),
  "config",
  "mcp-servers.json",
);

function getEncKey(): Buffer | null {
  const raw = process.env.ENCRYPTION_KEY?.trim();
  if (!raw) return null;
  return crypto.createHash("sha256").update(raw).digest();
}

function enc(v: string): string {
  const key = getEncKey();
  if (!key) return v;
  const iv = crypto.randomBytes(12);
  const c = crypto.createCipheriv("aes-256-gcm", key, iv);
  const ct = Buffer.concat([c.update(v, "utf8"), c.final()]);
  const tag = c.getAuthTag();
  return `enc:v1:${iv.toString("base64")}:${tag.toString("base64")}:${ct.toString("base64")}`;
}

function dec(v: string): string {
  const key = getEncKey();
  if (!key || !v?.startsWith("enc:v1:")) return v;
  const [, , ivB64, tagB64, ctB64] = v.split(":");
  const d = crypto.createDecipheriv("aes-256-gcm", key, Buffer.from(ivB64, "base64"));
  d.setAuthTag(Buffer.from(tagB64, "base64"));
  return Buffer.concat([d.update(Buffer.from(ctB64, "base64")), d.final()]).toString("utf8");
}

function encryptConfigItems(configs: MCPServerConfig[]): MCPServerConfig[] {
  return configs.map((c) => ({
    ...c,
    env: c.env
      ? Object.fromEntries(Object.entries(c.env).map(([k, v]) => [/token|secret|key|password/i.test(k) ? k : k, /token|secret|key|password/i.test(k) ? enc(v) : v]))
      : c.env,
  }));
}

function decryptConfigItems(configs: MCPServerConfig[]): MCPServerConfig[] {
  return configs.map((c) => ({
    ...c,
    env: c.env ? Object.fromEntries(Object.entries(c.env).map(([k, v]) => [k, dec(v)])) : c.env,
  }));
}

export class MCPClientManager {
  private servers: Map<string, MCPServerConnection> = new Map();
  private configPath: string;

  constructor(configPath?: string) {
    this.configPath = configPath || DEFAULT_MCP_CONFIG_PATH;
  }

  /** Load server configs from file */
  loadConfig(): MCPServerConfig[] {
    try {
      if (fs.existsSync(this.configPath)) {
        const raw = fs.readFileSync(this.configPath, "utf-8");
        const configs = JSON.parse(raw) as MCPServerConfig[];
        return decryptConfigItems(configs);
      }
    } catch { /* ignore */ }
    return [];
  }

  /** Save server configs to file */
  saveConfig(configs: MCPServerConfig[]): void {
    try {
      const dir = path.dirname(this.configPath);
      if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(this.configPath, JSON.stringify(encryptConfigItems(configs), null, 2), "utf-8");
    } catch { /* best-effort */ }
  }

  /** Get all registered server configs */
  getConfigs(): MCPServerConfig[] {
    return this.loadConfig();
  }

  /** Register a new server config */
  register(config: MCPServerConfig): void {
    const configs = this.loadConfig();
    const idx = configs.findIndex((c) => c.name === config.name);
    if (idx >= 0) {
      configs[idx] = config;
    } else {
      configs.push(config);
    }
    this.saveConfig(configs);
  }

  /** Remove a server config */
  unregister(name: string): boolean {
    const configs = this.loadConfig();
    const len = configs.length;
    const filtered = configs.filter((c) => c.name !== name);
    if (filtered.length !== len) {
      this.saveConfig(filtered);
      this.disconnect(name);
      return true;
    }
    return false;
  }

  /** Connect to a server */
  async connect(name: string): Promise<MCPServerConnection> {
    const existing = this.servers.get(name);
    if (existing) return existing;

    const configs = this.loadConfig();
    const config = configs.find((c) => c.name === name);
    if (!config) throw new Error(`MCP server "${name}" not found in config`);

    const connection = new MCPServerConnection(config);
    await connection.initialize();
    this.servers.set(name, connection);
    return connection;
  }

  /** Connect to all auto-start servers */
  async connectAll(): Promise<MCPServerConnection[]> {
    const configs = this.loadConfig();
    const results: MCPServerConnection[] = [];
    for (const cfg of configs) {
      if (cfg.autoStart !== false) {
        try {
          const conn = await this.connect(cfg.name);
          results.push(conn);
        } catch (err: any) {
          console.warn(`MCP auto-connect "${cfg.name}" failed: ${err.message}`);
        }
      }
    }
    return results;
  }

  /** Disconnect from a server */
  disconnect(name: string): void {
    const conn = this.servers.get(name);
    if (conn) {
      conn.close();
      this.servers.delete(name);
    }
  }

  /** Disconnect all */
  disconnectAll(): void {
    for (const [name] of this.servers) {
      this.disconnect(name);
    }
  }

  /** Get all connected servers */
  getConnections(): MCPServerConnection[] {
    return Array.from(this.servers.values());
  }

  /** Get all tools from all connected servers */
  getAllTools(): MCPTool[] {
    const tools: MCPTool[] = [];
    for (const conn of this.servers.values()) {
      tools.push(...conn.tools);
    }
    return tools;
  }
}

// Singleton for the agent-service instance
export const globalMCPManager = new MCPClientManager();
