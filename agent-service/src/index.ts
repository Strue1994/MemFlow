import express, { Request, Response as ExpressResponse, NextFunction } from "express";
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
import { createAgentLoop } from "./agent-loop";
import { createExecutorTools, createMemoryTools, createSkillTools } from "./agent-tools";
import { SkillManager } from "./skill-system";
import { MCPServer } from "./mcp-server";
import { GatewayBridge } from "./gateway-bridge";
import { authMiddleware } from "./auth-middleware";

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
  app.use(express.json({ limit: "1mb" }));
app.use(authMiddleware);
  app.use(express.urlencoded({ limit: "1mb", extended: false }));

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
      const { text } = req.body as { text: string };
      if (!text) { res.status(400).json({ error: "text is required" }); return; }
      const loop = await createAgentLoop();
      const exTools = createExecutorTools(EXECUTOR_URL, EXECUTOR_API_KEY);
      const memTools = createMemoryTools(MEMORY_HUB_URL);
      for (const t of [...exTools, ...memTools, ...createSkillTools()]) { loop.addTool(t); }
      const result = await loop.run(text);
      if (result.success) {
        const sm = new SkillManager();
        sm.generateSkill({ workflowId: "agent_"+Date.now(), taskText: text, steps: [], success: true, durationMs: 0, timestamp: new Date().toISOString() });
      }
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

  return app;
}

const app = createApp();
const PORT = process.env.PORT || 3000;

if (require.main === module) {
  void (async () => {
    const llmSettings = await getLLMSettings();
    app.listen(PORT, () => {
      console.log(`Agent service ready at http://127.0.0.1:${PORT}`);
      console.log(`Agent service running on port ${PORT}`);
      console.log(`Executor URL: ${EXECUTOR_URL}`);
      console.log(`Executor API key configured: ${EXECUTOR_API_KEY ? "yes" : "no"}`);
      console.log(`OpenAI API key configured: ${llmSettings.apiKey ? "yes" : "no"}`);
      console.log(`OpenAI base URL configured: ${llmSettings.baseUrl ? llmSettings.baseUrl : "default"}`);
      console.log(`OpenAI model: ${llmSettings.model}`);
    });
  })().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}

