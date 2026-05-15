import express, { Request, Response as ExpressResponse, NextFunction } from "express";
import cors from "cors";
import { getWorkflow, listWorkflows, Workflow } from "./workflowRegistry";
import { attachAutonomyRoutes } from "./autonomy_supervisor";
import { attachComputerRoutes } from "./computer_agent";
import { attachCodingKernelRoutes } from "./coding_kernel";
import { createChatCompletion } from "./llm_client";
import { getLLMSettings, LLM_PROVIDER_PRESETS, saveLLMSettings, type LLMSettings } from "./llm_settings";
import { executeTaskEntry, type TaskEntryDeps } from "./task_entry";
import { workflowCreator } from "./messaging/nlp_workflow_creator";
import { FileTaskHistoryStore } from "./task_router/history_store";
import { decideTaskRoute } from "./task_router/router";
import { FileWorkflowMetadataStore } from "./task_router/workflow_metadata_store";
import type { HistoricalTaskRecord, WorkflowAssetMetadata } from "./task_router/types";
import path from "node:path";
import runAgent from "./agent-loop";
import { createExecutorTools, createMemoryTools, createSkillTools } from "./agent-tools";
import { SkillManager } from "./skill-system";
import { MCPServer } from "./mcp-server";
import { getRouterConfig, setRouterConfig, globalCostTracker, getConfiguredProviders } from "./llm_router";
import { getSetupStatus as getProvSetupStatus } from "./provider-config";
import { scanSkillFiles, loadSkillFile, importSkillsFromDir, discoverAllSkills } from "./skill-loader";
import { globalMCPManager, type MCPServerConfig } from "./mcp-client";
import { globalCurator } from "./curator";
import { compressContext } from "./context-compressor";
import { runBeforePipeline, runAfterPipeline, globalMiddleware, type MiddlewareContext } from "./middleware-chain";
import * as checkpoints from "./checkpoints";
import * as marketplace from "./marketplace";
import * as agentConfig from "./agent-config";
import { globalTracer, traceAgent } from "./tracing";
import { runScan } from "./security-scanner";
import { GatewayBridge } from "./gateway-bridge";
import { authMiddleware } from "./auth-middleware";
import { rateLimiter } from "./rate-limiter";
import { validateAgentExecuteBody, validateChatCompletionsBody, validateCheckpointSaveBody } from "./validate";
import { createBackup, restoreBackup } from "./backup";
import { logger } from "./logger";
import { httpRequestsTotal, agentExecuteDuration, activeSessions, mcpConnectionsActive, metricsText } from "./metrics";
import { setGoal, getActiveGoal, completeGoal, updateGoalProgress, formatGoalPrompt, listGoals } from "./goal-loop";

function sanitizeServiceBaseUrl(value: string): string {
  return value.trim().replace(/\s+/g, "").replace(/\/+$/, "");
}

const EXECUTOR_API_KEY = process.env.EXECUTOR_API_KEY || "memflow-local-dev-key";
const EXECUTOR_URL = sanitizeServiceBaseUrl(process.env.EXECUTOR_URL || "http://127.0.0.1:8082");
const MEMORY_HUB_URL = sanitizeServiceBaseUrl(process.env.MEMORY_HUB_URL || "http://127.0.0.1:8081");

interface WorkflowSummary {
  id: string;
  name?: string;
  version?: number;
  nodes?: number;
}

interface MemoryEntry {
  id: string;
  content: string;
  memory_type: string;
  importance: number;
  last_access: number;
  created_at: number;
}

interface ExecuteRequest {
  workflowId: string;
  params?: Record<string, unknown>;
  version?: number;
  timeout_seconds?: number;
}

interface ChatRequest {
  text: string;
}

interface CreateWorkflowRequest {
  name?: string;
  n8n_json: object;
}

interface UpdateLLMSettingsRequest {
  provider?: LLMSettings["provider"];
  apiKey?: string;
  baseUrl?: string;
  model?: string;
}

interface TestLLMSettingsRequest extends UpdateLLMSettingsRequest {
  prompt?: string;
}

interface TaskExecuteRequest {
  text: string;
  params?: Record<string, unknown>;
  version?: number;
  timeout_seconds?: number;
}

interface AppDependencies {
  historyStore?: {
    list(): Promise<HistoricalTaskRecord[]>;
    append(record: HistoricalTaskRecord): Promise<void>;
  };
  workflowMetadataStore?: {
    list(): Promise<WorkflowAssetMetadata[]>;
    upsert(record: WorkflowAssetMetadata): Promise<void>;
  };
  executeWorkflow?: (
    workflowId: string,
    params?: Record<string, unknown>,
    version?: number,
    timeoutSeconds?: number,
  ) => Promise<unknown>;
  executeAgent?: (text: string) => Promise<unknown>;
  generateWorkflow?: (text: string) => Promise<{ workflowId: string; message?: string }>;
}

function validateBody<T>(_schema?: unknown) {
  return (_req: Request, _res: ExpressResponse, next: NextFunction) => {
    next();
  };
}

export function buildExecutorHeaders(includeJson = false): Record<string, string> {
  const headers: Record<string, string> = {};
  if (includeJson) {
    headers["Content-Type"] = "application/json";
  }
  if (EXECUTOR_API_KEY) {
    headers["X-API-Key"] = EXECUTOR_API_KEY;
  }
  return headers;
}

function requireExecutorApiKey(): void {
  if (!EXECUTOR_API_KEY) {
    throw new Error("EXECUTOR_API_KEY is not configured");
  }
}

async function searchMemory(query: string, k = 5): Promise<MemoryEntry[]> {
  try {
    const response = await fetch(`${MEMORY_HUB_URL}/memories/search?q=${encodeURIComponent(query)}&k=${k}`);
    if (!response.ok) return [];
    return response.json();
  } catch {
    return [];
  }
}

async function storeMemory(content: string, memoryType: string, importance = 0.5, metadata?: object): Promise<void> {
  try {
    await fetch(`${MEMORY_HUB_URL}/memories`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ content, type: memoryType, importance, metadata }),
    });
  } catch {
    // Memory is a best-effort side channel.
  }
}

function getLocalWorkflows(): WorkflowSummary[] {
  return listWorkflows().map((id) => {
    const workflow = getWorkflow(id);
    return {
      id,
      name: id,
      version: 1,
      nodes: workflow?.nodes.length || 0,
    };
  });
}

export async function loadAvailableWorkflows(): Promise<WorkflowSummary[]> {
  if (!EXECUTOR_API_KEY) {
    return getLocalWorkflows();
  }

  const response = await fetch(`${EXECUTOR_URL}/workflows`, {
    headers: buildExecutorHeaders(),
  });

  if (!response.ok) {
    throw new Error(await readExecutorError(response));
  }

  const workflows = (await response.json()) as Array<{ id: string; name?: string; version?: number }>;
  return workflows.map((workflow) => ({
    id: workflow.id,
    name: workflow.name,
    version: workflow.version,
  }));
}

async function loadWorkflowById(id: string): Promise<Workflow | Record<string, unknown> | undefined> {
  if (EXECUTOR_API_KEY) {
    const response = await fetch(`${EXECUTOR_URL}/workflow/${encodeURIComponent(id)}`, {
      headers: buildExecutorHeaders(),
    });

    if (response.status === 404) {
      return undefined;
    }

    if (!response.ok) {
      throw new Error(await readExecutorError(response));
    }

    return (await response.json()) as Record<string, unknown>;
  }

  return getWorkflow(id);
}

async function readExecutorError(response: globalThis.Response): Promise<string> {
  try {
    const error = (await response.json()) as { error?: string };
    return error.error || `HTTP ${response.status}`;
  } catch {
    return `HTTP ${response.status}`;
  }
}

export async function executeWorkflow(workflowId: string, params: Record<string, unknown> = {}, version?: number, timeoutSeconds?: number) {
  requireExecutorApiKey();
  const response = await fetch(`${EXECUTOR_URL}/execute`, {
    method: "POST",
    headers: buildExecutorHeaders(true),
    body: JSON.stringify({
      workflow_id: workflowId,
      params,
      version,
      timeout_seconds: timeoutSeconds,
    }),
  });

  if (!response.ok) {
    throw new Error(await readExecutorError(response));
  }

  return response.json();
}

const runtimeRoot = process.env.MEMFLOW_RUNTIME_ROOT
  ? path.resolve(process.env.MEMFLOW_RUNTIME_ROOT)
  : path.resolve(__dirname, "..", "..", ".memflow-runtime");

const defaultHistoryStore = new FileTaskHistoryStore(
  path.join(runtimeRoot, "data", "task-history.json"),
);
const defaultWorkflowMetadataStore = new FileWorkflowMetadataStore(
  path.join(runtimeRoot, "data", "workflow-assets.json"),
);

async function executeAgentTask(text: string): Promise<unknown> {
  const llmSettings = await getLLMSettings();

  if (!llmSettings.apiKey) {
    throw new Error("LLM API key is not configured");
  }

  return createChatCompletion(llmSettings, [
    {
      role: "system",
      content: "You are a one-off execution agent. Complete the user's task directly and reply with plain text.",
    },
    {
      role: "user",
      content: text,
    },
  ]);
}

function createTaskEntryDeps(overrides: AppDependencies = {}): TaskEntryDeps & Required<Pick<AppDependencies, "historyStore" | "workflowMetadataStore">> {
  const historyStore = overrides.historyStore || defaultHistoryStore;
  const workflowMetadataStore = overrides.workflowMetadataStore || defaultWorkflowMetadataStore;

  return {
    historyStore,
    workflowMetadataStore,
    executeWorkflow: overrides.executeWorkflow || executeWorkflow,
    executeAgent: overrides.executeAgent || executeAgentTask,
    generateWorkflow: overrides.generateWorkflow || ((text: string) => workflowCreator.createFromNaturalLanguage("web", "local-user", text)),
  };
}


// ---- Resumable session state ----
let restoredContext: any[] | null = null;

// ---- Start MCP Server (T1.3) ----
const mcpTransport = process.env.MCP_TRANSPORT;
if (mcpTransport === "sse" || mcpTransport === "stdio") {
  const mcpServer = new MCPServer({
    transport: mcpTransport,
    port: parseInt(process.env.MCP_PORT || "3301", 10),
    executorUrl: EXECUTOR_URL,
    memoryHubUrl: MEMORY_HUB_URL,
  });
  mcpServer.start();
}

export function createApp(deps: AppDependencies = {}) {
  const app = express();
  const taskEntryDeps = createTaskEntryDeps(deps);
  app.use(cors({ origin: process.env.CORS_ORIGIN || "http://localhost:3000" }));
  app.use(express.json({ limit: "1mb" }));
  app.use(express.urlencoded({ limit: "1mb", extended: false }));

  // request metrics + lightweight logging
  app.use((req, res, next) => {
    const started = Date.now();
    res.on("finish", () => {
      const ms = Date.now() - started;
      httpRequestsTotal.inc({ method: req.method, path: req.path, status: String(res.statusCode) });
      logger.info({ method: req.method, path: req.path, status: res.statusCode, duration_ms: ms }, "http_request");
    });
    next();
  });

  // rate limiter before auth to protect auth path itself
  app.use(rateLimiter);

  // Serve built web UI (from web-ui/dist/) as static files (before auth)
  const WEB_UI_DIR = path.resolve(__dirname, "..", "..", "web-ui", "dist");
  app.use(express.static(WEB_UI_DIR));

  app.use(authMiddleware);

  // SPA fallback for web UI (only if auth passes OR endpoint is public)
  const fs = require("fs");
  app.get(/^\/(?!api\/|health|ready|live|skills|providers|channels|agent\/|v1\/|checkpoints|curator|middleware|marketplace|metrics|goals|backup|restore|setup|status)/, (req, res, next) => {
    const indexPath = path.join(WEB_UI_DIR, "index.html");
    if (fs.existsSync(indexPath)) {
      res.sendFile(indexPath);
    } else {
      next();
    }
  });

  // health probes
  app.get("/health", (_req, res) => {
    res.json({ status: "ok", uptime_s: Math.floor(process.uptime()) });
  });

  app.get("/live", (_req, res) => {
    res.json({ live: true });
  });

  app.get("/ready", async (_req, res) => {
    try {
      const [ex, mem] = await Promise.all([
        fetch(`${EXECUTOR_URL}/health`, { method: "GET" }).then((r) => r.ok).catch(() => false),
        fetch(`${MEMORY_HUB_URL}/stats`, { method: "GET" }).then((r) => r.ok).catch(() => false),
      ]);
      if (ex && mem) return res.json({ ready: true, executor: true, memory_hub: true });
      res.status(503).json({ ready: false, executor: ex, memory_hub: mem });
    } catch {
      res.status(503).json({ ready: false });
    }
  });

  app.get("/metrics", async (_req, res) => {
    res.setHeader("Content-Type", "text/plain; version=0.0.4; charset=utf-8");
    res.send(await metricsText());
  });

  app.get("/llm-settings", async (_req, res) => {
    try {
      const settings = await getLLMSettings();
      res.json(settings);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(500).json({ error: message });
    }
  });

  app.get("/llm-settings/catalog", (_req, res) => {
    res.json({ providers: LLM_PROVIDER_PRESETS });
  });

  app.post("/llm-settings", async (req, res) => {
    try {
      const nextSettings = req.body as UpdateLLMSettingsRequest;
      const saved = await saveLLMSettings(nextSettings);
      res.json(saved);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(400).json({ error: message });
    }
  });

  app.post("/llm-settings/test", async (req, res) => {
    try {
      const overrides = req.body as TestLLMSettingsRequest;
      const current = await getLLMSettings();
      const effective = {
        ...current,
        ...overrides,
      };
      const result = await createChatCompletion(effective, [
        { role: "system", content: "Reply with one short sentence and no markdown." },
        { role: "user", content: overrides.prompt?.trim() || "Respond with exactly: ok" },
      ]);
      res.json({
        success: true,
        provider: effective.provider,
        model: effective.model,
        content: result.content,
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(400).json({ error: message });
    }
  });

  app.post("/execute", validateBody<ExecuteRequest>(), async (req, res) => {
    try {
      const { workflowId, params = {}, version, timeout_seconds } = req.body as ExecuteRequest;

      if (!workflowId) {
        res.status(400).json({ error: "workflowId is required" });
        return;
      }

      const result = await executeWorkflow(workflowId, params, version, timeout_seconds);
      res.json({ success: true, result });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(500).json({ error: message });
    }
  });

  app.post("/chat", validateBody<ChatRequest>(), async (req, res) => {
    try {
      const { text } = req.body as ChatRequest;

      if (!text) {
        res.status(400).json({ error: "text is required" });
        return;
      }

      const llmSettings = await getLLMSettings();

      if (!llmSettings.apiKey) {
        res.status(503).json({ error: "LLM API key is not configured" });
        return;
      }

      const relevantMemories = await searchMemory(text, 3);
      const availableWorkflows = await loadAvailableWorkflows();

      let contextFromMemory = "";
      if (relevantMemories.length > 0) {
        contextFromMemory = `\nRelevant memories from past interactions:\n${relevantMemories.map((m) => `- ${m.content}`).join("\n")}`;
      }

      const completion = await createChatCompletion(llmSettings, [
        {
          role: "system",
          content: `You are a workflow matcher. Given user input, identify which workflow to use and extract parameters.
Available workflows: ${JSON.stringify(availableWorkflows)}
Respond with JSON only: { "workflowId": "string", "params": { ... } }`,
        },
        {
          role: "user",
          content: `${text}${contextFromMemory}\n\nReturn valid JSON only.`,
        },
      ]);

      const response = JSON.parse(completion.content || "{}");
      const { workflowId, params } = response;

      if (!workflowId) {
        res.status(400).json({ error: "Could not match input to a workflow" });
        return;
      }

      const result = await executeWorkflow(workflowId, params || {});

      await storeMemory(
        `User: "${text}" -> Workflow: ${workflowId}, Params: ${JSON.stringify(params || {})}, Result: ${JSON.stringify(result).slice(0, 200)}`,
        "Conversation",
        0.6,
        { workflowId }
      );

      res.json({ success: true, result, matched: response, memories: relevantMemories });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(500).json({ error: message });
    }
  });

  app.post("/create_workflow", validateBody<CreateWorkflowRequest>(), async (req, res) => {
    try {
      requireExecutorApiKey();
      const { name, n8n_json } = req.body as CreateWorkflowRequest;

      if (!n8n_json) {
        res.status(400).json({ error: "n8n_json is required" });
        return;
      }

      const response = await fetch(`${EXECUTOR_URL}/compile`, {
        method: "POST",
        headers: buildExecutorHeaders(true),
        body: JSON.stringify({
          name: name || undefined,
          n8n_json,
        }),
      });

      if (!response.ok) {
        throw new Error(await readExecutorError(response));
      }

      const result = await response.json();
      res.json({ success: true, ...result });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(500).json({ error: message });
    }
  });

  app.get("/workflows", async (_req, res) => {
    try {
      const workflows = await loadAvailableWorkflows();
      res.json({ workflows });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(502).json({ error: message });
    }
  });

  app.get("/workflows/:id", async (req, res) => {
    try {
      const workflow = await loadWorkflowById(req.params.id);
      if (!workflow) {
        res.status(404).json({ error: "Workflow not found" });
        return;
      }
      res.json(workflow);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(502).json({ error: message });
    }
  });

  app.post("/tasks/execute", validateBody<TaskExecuteRequest>(), async (req, res) => {
    try {
      const { text, params = {}, version, timeout_seconds } = req.body as TaskExecuteRequest;

      if (!text) {
        res.status(400).json({ error: "text is required" });
        return;
      }

      const [workflows, history] = await Promise.all([
        taskEntryDeps.workflowMetadataStore.list(),
        taskEntryDeps.historyStore.list(),
      ]);
      const routeDecision = decideTaskRoute({
        taskText: text,
        workflows,
        history,
      });

      const result = await executeTaskEntry(
        {
          text,
          params,
          routeDecision,
          version,
          timeoutSeconds: timeout_seconds,
        },
        taskEntryDeps,
      );

      res.json(result);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(500).json({ error: message });
    }
  });

  app.get("/tasks/history", async (_req, res) => {
    try {
      res.json({ items: await taskEntryDeps.historyStore.list() });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(500).json({ error: message });
    }
  });

  attachAutonomyRoutes(app, {
    executorUrl: EXECUTOR_URL,
    buildExecutorHeaders,
    listWorkflows: loadAvailableWorkflows,
    remember: async (content, metadata) => {
      await storeMemory(content, "AutonomyReflection", 0.7, metadata);
    },
    recall: async (query, k = 3) => {
      const entries = await searchMemory(query, k);
      return entries.map((entry) => entry.content);
    },
  });
  attachComputerRoutes(app);
  attachCodingKernelRoutes(app);

  app.post("/agent/execute", async (req, res) => {
    try {
      const t0 = Date.now();
      const errBody = validateAgentExecuteBody(req.body);
      if (errBody) { res.status(400).json({ error: errBody }); return; }
      const { text, stream } = req.body as { text: string; stream?: boolean };
      if (!text) { res.status(400).json({ error: "text is required" }); return; }

      // Tracing
      const sessionId = `session_${Date.now()}`;
      const traceSpan = traceAgent(sessionId, text);

      const exTools = createExecutorTools(EXECUTOR_URL, EXECUTOR_API_KEY);

      // Run before middleware pipeline
      const ctx: MiddlewareContext = {
        text,
        messages: restoredContext ? [...restoredContext] : [],
        tools: exTools,
        stream,
        meta: { sessionId },
      };
      // Clear restored context after first use (it's now in the conversation flow)
      restoredContext = null;
      const { modifiedCtx, earlyResponse } = await runBeforePipeline(ctx);
      if (earlyResponse) {
        globalTracer.completeSpan(traceSpan, earlyResponse);
        res.json({ success: true, output: earlyResponse, middleware: true });
        return;
      }

      const result = await runAgent(modifiedCtx.text, { tools: modifiedCtx.tools, stream: modifiedCtx.stream });
      globalTracer.completeSpan(traceSpan, result.output || "", result.error);

      // Run after middleware pipeline
      if (result.success && result.output) {
        result.output = await runAfterPipeline(modifiedCtx, result.output);
      }

      // Record execution for curator self-learning
      globalCurator.recordExecution({
        workflowId: "agent_" + Date.now(),
        taskText: text,
        steps: result.output ? ["Agent processed"] : [],
        success: result.success,
        durationMs: 0,
        timestamp: new Date().toISOString(),
      });

      agentExecuteDuration.observe((Date.now() - t0) / 1000);

      res.json(result);
    } catch (e) { res.status(500).json({ error: (e instanceof Error ? e.message : String(e)) }); }
  });

  app.get("/skills", (_req, res) => { res.json({ skills: new SkillManager().listSkills() }); });

  app.post("/skills", (req, res) => {
    const { name, desc, pattern } = req.body as any;
    if (!name || !desc) { res.status(400).json({ error: "name and desc required" }); return; }
    const skill = new SkillManager().generateSkill({
      workflowId: "manual_"+Date.now(), taskText: desc,
      steps: pattern ? pattern.split(" -> ") : [], success: true, durationMs: 0,
      timestamp: new Date().toISOString()
    });
    res.json({ success: true, skill });
  });

  app.post("/gateway/message", async (req, res) => {
    try {
      const p = req.body as any;
      if (!p.platform || !p.text) { res.status(400).json({ error: "platform and text required" }); return; }
      const bg = new GatewayBridge({ executorUrl: EXECUTOR_URL, executorApiKey: EXECUTOR_API_KEY, memoryHubUrl: MEMORY_HUB_URL });
      const r = await bg.handleIncomingMessage(p.platform, p.channel_id, p.user_id, p.text);
      res.json({ response: r });
    } catch (e) { res.status(500).json({ error: (e instanceof Error ? e.message : String(e)) }); }
  });

  // OpenAI-compatible API (Hermes/OpenClaw-aligned)
  app.post("/v1/chat/completions", async (req, res) => {
    try {
      const errBody = validateChatCompletionsBody(req.body);
      if (errBody) { res.status(400).json({ error: errBody }); return; }
      const { model, messages, stream } = req.body as any;
      if (!messages) { res.status(400).json({ error: "messages required" }); return; }
      const lastUserMsg = messages.filter((m: any) => m.role === "user").pop();
      if (!lastUserMsg) { res.status(400).json({ error: "no user message" }); return; }
      const result = await runAgent(lastUserMsg.content, { stream });
      if (stream) {
        res.writeHead(200, { "Content-Type": "text/event-stream", "Cache-Control": "no-cache" });
        res.write("data: " + JSON.stringify({ choices: [{ delta: { content: result.output }, index: 0 }] }) + "\n\n");
        res.write("data: [DONE]\n");
        res.end();
      } else {
        res.json({
          id: "chatcmpl-" + Date.now(),
          object: "chat.completion",
          model: model || result.model,
          choices: [{ message: { role: "assistant", content: result.output }, index: 0 }],
          usage: { prompt_tokens: result.tokensUsed.input, completion_tokens: result.tokensUsed.output },
        });
      }
    } catch (e) { res.status(500).json({ error: (e instanceof Error ? e.message : String(e)) }); }
  });

  // ---- Smart Router API ----
  app.get("/router/config", (_req, res) => {
    res.json({ config: getRouterConfig(), providers: getConfiguredProviders() });
  });

  app.post("/router/config", (req, res) => {
    const cfg = setRouterConfig(req.body);
    res.json({ config: cfg });
  });

  app.get("/router/stats", (_req, res) => {
    res.json(globalCostTracker.getStats());
  });

  // ---- Curator API (Self-Learning Loop) ----
  app.post("/curator/run", async (_req, res) => {
    try {
      const report = await globalCurator.runCycle();
      res.json({ report });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.get("/curator/status", (_req, res) => {
    try {
      res.json(globalCurator.getStatus());
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.get("/curator/history", (_req, res) => {
    try {
      res.json({ records: globalCurator.getExecutionHistory().slice(-20) });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // ---- Context Compression API ----
  app.post("/compact", async (req, res) => {
    try {
      const { messages } = req.body as { messages?: any[] };
      if (!messages || !Array.isArray(messages)) {
        res.status(400).json({ error: "messages array required" });
        return;
      }
      const result = await compressContext(messages);
      res.json({
        messages: result.messages,
        results: result.results,
        savings: result.results.reduce((s, r) => s + r.savings, 0),
      });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // ---- Checkpoints API (Hermes-aligned state persistence) ----
  app.get("/checkpoints", (_req, res) => {
    try {
      const all = checkpoints.listCheckpoints();
      const stats = checkpoints.getStorageStats();
      res.json({ checkpoints: all, stats, hasResumable: checkpoints.hasResumableSession() });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.get("/checkpoints/latest", (req, res) => {
    try {
      const sessionId = (req.query as any).sessionId || undefined;
      const ck = checkpoints.getLatestCheckpoint(sessionId);
      if (!ck) { res.status(404).json({ error: "no checkpoint found" }); return; }
      res.json({ checkpoint: ck });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/checkpoints/save", (req, res) => {
    try {
      const errBody = validateCheckpointSaveBody(req.body);
      if (errBody) { res.status(400).json({ error: errBody }); return; }
      const { sessionId, messages } = req.body as { sessionId: string; messages: any[] };
      if (!sessionId || !messages) { res.status(400).json({ error: "sessionId and messages required" }); return; }
      const ck = checkpoints.saveCheckpoint(sessionId, messages);
      res.json({ checkpoint: ck });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.delete("/checkpoints/:sessionId", (req, res) => {
    try {
      const ok = checkpoints.deleteSession(req.params.sessionId);
      res.json({ deleted: ok });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/backup", (_req, res) => {
    try {
      const out = createBackup();
      res.json(out);
    } catch (e: any) {
      res.status(500).json({ error: e.message });
    }
  });

  app.post("/backup/restore", (req, res) => {
    try {
      const { path } = req.body as { path?: string };
      if (!path) { res.status(400).json({ error: "path required" }); return; }
      res.json(restoreBackup(path));
    } catch (e: any) {
      res.status(500).json({ error: e.message });
    }
  });

  // ---- Middleware Chain API ----
  app.get("/middleware/config", (_req, res) => {
    try {
      const middlewares = globalMiddleware.list().map((mw) => ({
        name: mw.name,
        description: mw.description,
        enabled: mw.enabled,
        priority: mw.priority,
      }));
      res.json({ middlewares });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/middleware/config/:name/toggle", (req, res) => {
    try {
      globalMiddleware.enable(req.params.name, req.body?.enabled !== false);
      const mw = globalMiddleware.get(req.params.name);
      res.json({ name: req.params.name, enabled: mw?.enabled ?? false });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // ---- Subagent API (proxied to executor) ----
  app.post("/subagents/spawn", async (req, res) => {
    try {
      const resp = await fetch(`${EXECUTOR_URL}/subagents/spawn`, {
        method: "POST",
        headers: { "Content-Type": "application/json", "X-API-Key": EXECUTOR_API_KEY },
        body: JSON.stringify(req.body),
      });
      const data = resp.ok ? await resp.json() : { error: await resp.text() };
      res.status(resp.status).json(data);
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.get("/subagents/:id/status", async (req, res) => {
    try {
      const resp = await fetch(`${EXECUTOR_URL}/subagents/${req.params.id}/status`, {
        headers: { "X-API-Key": EXECUTOR_API_KEY },
      });
      const data = resp.ok ? await resp.json() : { error: await resp.text() };
      res.status(resp.status).json(data);
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/subagents/:id/cancel", async (req, res) => {
    try {
      const resp = await fetch(`${EXECUTOR_URL}/subagents/${req.params.id}/cancel`, {
        method: "POST",
        headers: { "X-API-Key": EXECUTOR_API_KEY },
      });
      const data = resp.ok ? await resp.json() : { error: await resp.text() };
      res.status(resp.status).json(data);
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.get("/subagents", async (_req, res) => {
    try {
      const resp = await fetch(`${EXECUTOR_URL}/subagents`, {
        headers: { "X-API-Key": EXECUTOR_API_KEY },
      });
      const data = resp.ok ? await resp.json() : { error: await resp.text() };
      res.status(resp.status).json(data);
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // ---- Sandbox API (proxied to executor) ----
  app.get("/sandbox/config", async (_req, res) => {
    try {
      const resp = await fetch(`${EXECUTOR_URL}/sandbox/config`, {
        headers: { "X-API-Key": EXECUTOR_API_KEY },
      });
      const data = resp.ok ? await resp.json() : { error: await resp.text() };
      res.status(resp.status).json(data);
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/sandbox/config", async (req, res) => {
    try {
      const resp = await fetch(`${EXECUTOR_URL}/sandbox/config`, {
        method: "POST",
        headers: { "Content-Type": "application/json", "X-API-Key": EXECUTOR_API_KEY },
        body: JSON.stringify(req.body),
      });
      const data = resp.ok ? await resp.json() : { error: await resp.text() };
      res.status(resp.status).json(data);
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/sandbox/execute", async (req, res) => {
    try {
      const resp = await fetch(`${EXECUTOR_URL}/sandbox/execute`, {
        method: "POST",
        headers: { "Content-Type": "application/json", "X-API-Key": EXECUTOR_API_KEY },
        body: JSON.stringify(req.body),
      });
      const data = resp.ok ? await resp.json() : { error: await resp.text() };
      res.status(resp.status).json(data);
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // ---- Marketplace API ----
  app.get("/marketplace/list", (_req, res) => {
    try {
      res.json({ listings: marketplace.listMarketplace(), installed: marketplace.getInstalled() });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.get("/marketplace/search", (req, res) => {
    try {
      const q = ((req.query as any).q || "").toString();
      res.json({ results: marketplace.searchMarketplace(q) });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/marketplace/install", async (req, res) => {
    try {
      const { id } = req.body as { id: string };
      if (!id) { res.status(400).json({ error: "id required" }); return; }
      const result = await marketplace.installSkill(id);
      res.json(result);
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/marketplace/uninstall", (req, res) => {
    try {
      const { id } = req.body as { id: string };
      if (!id) { res.status(400).json({ error: "id required" }); return; }
      const ok = marketplace.uninstallSkill(id);
      res.json({ deleted: ok });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // ---- Agent Config API ----
  app.get("/agents/config", (_req, res) => {
    try {
      res.json({ agents: agentConfig.listAgentConfigs() });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.get("/agents/config/:id", (req, res) => {
    try {
      const config = agentConfig.getAgentConfig(req.params.id);
      if (!config) { res.status(404).json({ error: "agent not found" }); return; }
      res.json({ agent: config });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/agents/config", (req, res) => {
    try {
      const config = req.body as agentConfig.AgentConfig;
      if (!config.id) { res.status(400).json({ error: "agent id required" }); return; }
      agentConfig.saveAgentConfig(config);
      res.json({ agent: config });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.delete("/agents/config/:id", (req, res) => {
    try {
      const ok = agentConfig.deleteAgentConfig(req.params.id);
      res.json({ deleted: ok });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // ---- Tracing / Observability API ----
  app.get("/traces", (_req, res) => {
    try {
      const sessions = globalTracer.listSessions();
      activeSessions.set(sessions.length);
      res.json({ sessions: sessions.length, sessionIds: sessions.slice(-20) });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.get("/traces/:sessionId", (req, res) => {
    try {
      const summary = globalTracer.getSummary(req.params.sessionId);
      if (!summary) { res.status(404).json({ error: "session not found" }); return; }
      res.json(summary);
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // ---- Security Scanner API ----
  app.post("/security/scan", (req, res) => {
    try {
      const { dir } = req.body as { dir?: string };
      const report = runScan(dir);
      res.json(report);
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // ---- Skills Import API (SKILL.md compatibility) ----
  app.get("/skills/imported", (_req, res) => {
    try {
      const sm = new SkillManager();
      const all = discoverAllSkills(sm);
      res.json({ skills: all, count: all.length });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/skills/import", (req, res) => {
    try {
      const { url, dir } = req.body as { url?: string; dir?: string };
      const sm = new SkillManager();
      let result: { imported: number; skills: any[] };

      if (dir) {
        result = importSkillsFromDir(dir, sm);
      } else if (url) {
        // URL import: download SKILL.md, load it
        res.status(400).json({ error: "URL import not yet implemented, use dir for local paths" });
        return;
      } else {
        // Auto-discover from all search paths
        result = { imported: 0, skills: discoverAllSkills(sm) };
        result.imported = result.skills.length;
      }

      res.json({ imported: result.imported, count: result.skills.length, skills: result.skills });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // ---- Provider Config API ----
  const {
    getProviders, addOrUpdateProvider, removeProvider, getSetupStatus,
    getProviderTemplates, getChannels, addOrUpdateChannel, removeChannel, getChannelTemplates,
  } = require("./provider-config");

  // Setup status (first-run wizard)
  app.get("/setup/status", async (_req, res) => {
    try {
      const status = await getSetupStatus();
      res.json({
        status,
        providerTemplates: getProviderTemplates(),
        channelTemplates: getChannelTemplates(),
      });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // Provider management
  app.get("/providers", async (_req, res) => {
    try { res.json({ providers: await getProviders() }); }
    catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/providers", async (req, res) => {
    try {
      const { id, ...updates } = req.body;
      if (!id) { res.status(400).json({ error: "provider id required" }); return; }
      const provider = await addOrUpdateProvider(id, updates);
      res.json({ provider });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.delete("/providers/:id", async (req, res) => {
    try {
      const ok = await removeProvider(req.params.id);
      res.json({ deleted: ok });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // Channel management
  app.get("/channels", async (_req, res) => {
    try { res.json({ channels: await getChannels() }); }
    catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/channels", async (req, res) => {
    try {
      const { id, ...updates } = req.body;
      if (!id) { res.status(400).json({ error: "channel id required" }); return; }
      const channel = await addOrUpdateChannel(id, updates);
      res.json({ channel });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.delete("/channels/:id", async (req, res) => {
    try {
      const ok = await removeChannel(req.params.id);
      res.json({ deleted: ok });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // ---- MCP Client API ----
  app.get("/mcp/servers", (_req, res) => {
    try {
      const configs = globalMCPManager.getConfigs();
      const connections = globalMCPManager.getConnections().map((c) => ({
        name: c.config.name,
        transport: c.config.transport,
        tools: c.tools.map((t) => ({ name: t.name, description: t.description })),
      }));
      mcpConnectionsActive.set(connections.length);
      res.json({ configs, connections });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/mcp/servers", (req, res) => {
    try {
      const config = req.body as MCPServerConfig;
      if (!config.name || !config.transport) {
        res.status(400).json({ error: "name and transport required" });
        return;
      }
      globalMCPManager.register(config);
      // Don't auto-connect — connect explicitly via POST /mcp/servers/:name/connect
      res.json({ status: "registered", name: config.name });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.delete("/mcp/servers/:name", (req, res) => {
    try {
      const ok = globalMCPManager.unregister(req.params.name);
      res.json({ deleted: ok });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/mcp/servers/:name/connect", async (req, res) => {
    try {
      await globalMCPManager.connect(req.params.name);
      res.json({ status: "connected", name: req.params.name });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/mcp/servers/:name/disconnect", (req, res) => {
    try {
      globalMCPManager.disconnect(req.params.name);
      res.json({ status: "disconnected", name: req.params.name });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  // ---- Goal Loop API ----
  app.get("/goal", (_req, res) => {
    try {
      const active = getActiveGoal();
      const all = listGoals();
      res.json({ active, all });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/goal/set", (req, res) => {
    try {
      const { text } = req.body as { text: string };
      if (!text) { res.status(400).json({ error: "text required" }); return; }
      const goal = setGoal(text);
      res.json({ goal });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.post("/goal/complete", (_req, res) => {
    try {
      const goal = completeGoal();
      res.json({ goal });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  app.get("/goal/prompt", (_req, res) => {
    try {
      const prompt = formatGoalPrompt();
      res.json({ prompt });
    } catch (e: any) { res.status(500).json({ error: e.message }); }
  });

  return app;
}

const app = createApp();
const PORT = process.env.PORT || 3000;

if (require.main === module) {
  void (async () => {
    const llmSettings = await getLLMSettings();
    const setupStatus = await getProvSetupStatus();

    // Auto-import SKILL.md skills on startup
    try {
      const sm = new SkillManager();
      const discovered = discoverAllSkills(sm);
      if (discovered.length > 0) {
        console.log(`✓ ${discovered.length} skill(s) loaded from SKILL.md files`);
      }
    } catch { /* best-effort */ }

    // Auto-connect MCP servers
    try {
      const count = globalMCPManager.getConfigs().length;
      if (count > 0) {
        await globalMCPManager.connectAll();
        const tools = globalMCPManager.getAllTools().length;
        console.log(`✓ ${count} MCP server(s) connected, ${tools} tool(s) available`);
      }
    } catch { /* best-effort */ }

    // Auto-resume from checkpoint
    try {
      if (checkpoints.hasResumableSession()) {
        const lastSession = checkpoints.getLastSessionId();
        const ck = checkpoints.getLatestCheckpoint();
        if (ck && ck.messages && ck.messages.length > 0) {
          restoredContext = ck.messages;
          const stats = checkpoints.getStorageStats();
          logger.info({ sessionId: lastSession, messages: ck.messages.length }, "Auto-resumed session from checkpoint");
          console.log(`✓ Auto-resumed session ${lastSession} (${ck.messages.length} messages restored)`);
        }
      }
    } catch { /* best-effort */ }

    const server = app.listen(PORT, () => {
      logger.info({ port: PORT }, `Agent service ready at http://127.0.0.1:${PORT}`);
      logger.info({ executorUrl: EXECUTOR_URL, hasExecutorApiKey: !!EXECUTOR_API_KEY }, "executor config");
      logger.info({ hasOpenAiKey: !!llmSettings.apiKey, baseUrl: llmSettings.baseUrl || "default", model: llmSettings.model }, "llm config");
      if (setupStatus.needsSetup) {
        logger.warn("Setup required: no LLM providers configured. Use /setup/status then /providers and /channels.");
      } else {
        logger.info({ providers: setupStatus.providers, channels: setupStatus.channels }, "setup status");
      }
    });

    const shutdown = async (signal: string) => {
      logger.warn({ signal }, "graceful shutdown start");
      try {
        globalMCPManager.disconnectAll();
      } catch {}
      try {
        checkpoints.saveCheckpoint("shutdown", [{ role: "system", content: `Shutdown via ${signal}` }]);
      } catch {}
      server.close(() => {
        logger.info("http server closed");
        process.exit(0);
      });
      setTimeout(() => process.exit(1), 10_000).unref();
    };

    process.on("SIGTERM", () => { void shutdown("SIGTERM"); });
    process.on("SIGINT", () => { void shutdown("SIGINT"); });
  })().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}




