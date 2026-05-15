import express from "express";
import { OpenAI } from "openai";
import { getWorkflow, listWorkflows, Workflow } from "./workflowRegistry";

const OPENAI_API_KEY = process.env.OPENAI_API_KEY;
const EXECUTOR_API_KEY = process.env.EXECUTOR_API_KEY;
const EXECUTOR_URL = process.env.EXECUTOR_URL || "http://127.0.0.1:8080";
const MEMORY_HUB_URL = process.env.MEMORY_HUB_URL || "http://127.0.0.1:8081";

const openai = OPENAI_API_KEY ? new OpenAI({ apiKey: OPENAI_API_KEY }) : null;

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

async function readExecutorError(response: Response): Promise<string> {
  try {
    const error = (await response.json()) as { error?: string };
    return error.error || `HTTP ${response.status}`;
  } catch {
    return `HTTP ${response.status}`;
  }
}

async function executeWorkflow(workflowId: string, params: Record<string, unknown> = {}, version?: number, timeoutSeconds?: number) {
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

export function createApp() {
  const app = express();
  app.use(express.json());

  app.post("/execute", async (req, res) => {
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

  app.post("/chat", async (req, res) => {
    try {
      const { text } = req.body as ChatRequest;

      if (!text) {
        res.status(400).json({ error: "text is required" });
        return;
      }

      if (!openai) {
        res.status(503).json({ error: "OPENAI_API_KEY is not configured" });
        return;
      }

      const relevantMemories = await searchMemory(text, 3);
      const availableWorkflows = await loadAvailableWorkflows();

      let contextFromMemory = "";
      if (relevantMemories.length > 0) {
        contextFromMemory = `\nRelevant memories from past interactions:\n${relevantMemories.map((m) => `- ${m.content}`).join("\n")}`;
      }

      const completion = await openai.chat.completions.create({
        model: "gpt-4o-mini",
        messages: [
          {
            role: "system",
            content: `You are a workflow matcher. Given user input, identify which workflow to use and extract parameters.
Available workflows: ${JSON.stringify(availableWorkflows)}
Respond with JSON only: { "workflowId": "string", "params": { ... } }`,
          },
          {
            role: "user",
            content: text + contextFromMemory,
          },
        ],
        response_format: { type: "json_object" },
      });

      const response = JSON.parse(completion.choices[0]?.message?.content || "{}");
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

  app.post("/create_workflow", async (req, res) => {
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

  return app;
}

const app = createApp();
const PORT = process.env.PORT || 3000;

if (require.main === module) {
  app.listen(PORT, () => {
    console.log(`Agent service running on port ${PORT}`);
    console.log(`Executor URL: ${EXECUTOR_URL}`);
    console.log(`Executor API key configured: ${EXECUTOR_API_KEY ? "yes" : "no"}`);
    console.log(`OpenAI API key configured: ${OPENAI_API_KEY ? "yes" : "no"}`);
  });
}
