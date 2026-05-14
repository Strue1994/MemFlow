# MemFlow Workflow-First Agent Platform Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first-stage MemFlow task console so natural-language requests are classified as repeatable or one-off, routed to workflow or agent execution, and returned through one unified, explainable result model.

**Architecture:** Keep `executor` as the workflow execution kernel and make `agent-service` the single entrypoint for task routing, workflow reuse, workflow generation, and result explanation. Reshape `web-ui` around a task-first console, using existing workflow generation and editor assets instead of building a second product path.

**Tech Stack:** TypeScript, Express, existing `agent-service` service modules, existing Rust executor HTTP API, React 18, Vite, Axios, Zustand, Vitest, React Testing Library

---

## Repository Note

`D:\文件\网站\strueauto.com\MemFlow` is not currently a git repository root. Each task still includes a commit step, but the command uses a PowerShell fallback that prints a checkpoint note when `.git` is absent. If a repo root is initialized later, the same step will create a real commit.

## File Structure

### Agent service files

- Create: `agent-service/src/task_router/types.ts`
- Create: `agent-service/src/task_router/scorer.ts`
- Create: `agent-service/src/task_router/history_store.ts`
- Create: `agent-service/src/task_router/workflow_metadata_store.ts`
- Create: `agent-service/src/task_router/router.ts`
- Create: `agent-service/src/task_router/router.test.ts`
- Create: `agent-service/src/task_entry.ts`
- Create: `agent-service/src/task_entry.test.ts`
- Modify: `agent-service/src/index.ts`
- Modify: `agent-service/src/index.test.ts`
- Modify: `agent-service/src/messaging/nlp_workflow_creator.ts`

### Web UI files

- Create: `web-ui/src/components/TaskConsole.tsx`
- Create: `web-ui/src/components/task-console/TaskRouteCard.tsx`
- Create: `web-ui/src/components/task-console/TaskExecutionTimeline.tsx`
- Create: `web-ui/src/components/task-console/TaskResultPanel.tsx`
- Create: `web-ui/src/components/WorkflowAssetsPage.tsx`
- Create: `web-ui/src/components/TaskHistoryPage.tsx`
- Create: `web-ui/src/components/__tests__/TaskConsole.test.tsx`
- Modify: `web-ui/package.json`
- Modify: `web-ui/src/App.tsx`
- Modify: `web-ui/src/api/client.ts`
- Modify: `web-ui/src/components/Layout.tsx`
- Modify: `web-ui/src/components/Dashboard.tsx`
- Modify: `web-ui/src/components/ExecutionLogs.tsx`
- Modify: `web-ui/src/components/NLCreator.tsx`

### Docs and cleanup

- Modify: `README.md`
- Delete: `web-ui/src/src/App.tsx`
- Delete: `web-ui/src/src/index.css`
- Delete: `web-ui/src/src/main.tsx`
- Delete: `web-ui/src/src/vite-env.d.ts`
- Delete: `web-ui/src/src/api/client.ts`
- Delete: `web-ui/src/src/components/CustomNode.tsx`
- Delete: `web-ui/src/src/components/DiffViewer.tsx`
- Delete: `web-ui/src/src/components/NodePalette.tsx`
- Delete: `web-ui/src/src/components/PropertyPanel.tsx`
- Delete: `web-ui/src/src/components/WorkflowEditor.tsx`
- Delete: `web-ui/src/src/stores/workflowStore.ts`

## Scope Check

This plan stays in one document because all tasks produce one working feature path: the Stage 1 task console. It deliberately postpones user-visible autonomy, marketplace-led workflows, and advanced optimization so the deliverable remains a single testable product slice instead of three disconnected subsystems.

### Task 1: Build the task-router domain with deterministic scoring

**Files:**
- Create: `agent-service/src/task_router/types.ts`
- Create: `agent-service/src/task_router/scorer.ts`
- Create: `agent-service/src/task_router/router.ts`
- Test: `agent-service/src/task_router/router.test.ts`

- [ ] **Step 1: Write the failing router tests**

```ts
import assert from "node:assert/strict";
import { decideTaskRoute } from "./router";
import type { HistoricalTaskRecord, WorkflowAssetMetadata } from "./types";

async function main() {
  const workflows: WorkflowAssetMetadata[] = [
    {
      workflowId: "wf_daily_orders",
      description: "Fetch pending orders, query delivery status, and email customers",
      inputHints: ["date", "order_source"],
      outputType: "summary",
      successRate: 0.94,
      reusable: true,
      examplePrompts: ["check pending orders and notify customers"],
      failureCategories: [],
      updatedAt: "2026-04-30T00:00:00.000Z",
    },
  ];

  const history: HistoricalTaskRecord[] = [
    {
      taskText: "check pending orders and notify customers",
      route: "workflow",
      workflowId: "wf_daily_orders",
      success: true,
      parameterKeys: ["date", "order_source"],
      outputType: "summary",
      createdAt: "2026-04-30T00:00:00.000Z",
    },
  ];

  const repeated = await decideTaskRoute({
    taskText: "check pending orders and notify customers again",
    workflows,
    history,
  });
  assert.equal(repeated.route, "workflow");
  assert.equal(repeated.workflowId, "wf_daily_orders");

  const oneOff = await decideTaskRoute({
    taskText: "look through these notes and tell me what is unusual",
    workflows,
    history,
  });
  assert.equal(oneOff.route, "agent");

  const generated = await decideTaskRoute({
    taskText: "nightly invoice export for april",
    workflows: [],
    history: [
      {
        taskText: "nightly invoice export",
        route: "workflow",
        success: true,
        parameterKeys: ["date_range"],
        outputType: "file",
        createdAt: "2026-04-30T00:00:00.000Z",
      },
    ],
  });
  assert.equal(generated.route, "generated_workflow");

  const uncertain = await decideTaskRoute({
    taskText: "run the monthly export",
    workflows: [],
    history: [],
  });
  assert.equal(uncertain.route, "clarification");
  assert.ok(uncertain.clarificationQuestion?.includes("missing"));

  console.log("task-router tests passed");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
```

- [ ] **Step 2: Run the router test to verify it fails**

Run: `cd agent-service; npm run build; node dist/task_router/router.test.js`

Expected: FAIL with `Cannot find module './router'` or missing export errors.

- [ ] **Step 3: Write the minimal router implementation**

```ts
// agent-service/src/task_router/types.ts
export type TaskRoute = "workflow" | "generated_workflow" | "agent" | "clarification";

export interface WorkflowAssetMetadata {
  workflowId: string;
  description: string;
  inputHints: string[];
  outputType: string;
  successRate: number;
  reusable: boolean;
  examplePrompts: string[];
  failureCategories: string[];
  updatedAt: string;
}

export interface HistoricalTaskRecord {
  taskText: string;
  route: Exclude<TaskRoute, "clarification">;
  workflowId?: string;
  success: boolean;
  parameterKeys: string[];
  outputType: string;
  createdAt: string;
}

export interface RoutingDecision {
  route: TaskRoute;
  repeatable: boolean;
  confidence: "high" | "medium" | "low";
  reason: string;
  workflowId?: string;
  clarificationQuestion?: string;
}

// agent-service/src/task_router/scorer.ts
const STOP_WORDS = new Set(["the", "a", "an", "and", "or", "to", "me", "these", "this", "again"]);

export function tokenize(text: string): string[] {
  return text
    .toLowerCase()
    .split(/[^a-z0-9\u4e00-\u9fa5]+/)
    .filter((token) => token && !STOP_WORDS.has(token));
}

export function overlapScore(left: string, right: string): number {
  const leftTokens = new Set(tokenize(left));
  const rightTokens = new Set(tokenize(right));
  const hits = [...leftTokens].filter((token) => rightTokens.has(token)).length;
  return hits === 0 ? 0 : hits / Math.max(leftTokens.size, rightTokens.size, 1);
}

// agent-service/src/task_router/router.ts
import { overlapScore } from "./scorer";
import type { HistoricalTaskRecord, RoutingDecision, WorkflowAssetMetadata } from "./types";

export async function decideTaskRoute(input: {
  taskText: string;
  workflows: WorkflowAssetMetadata[];
  history: HistoricalTaskRecord[];
}): Promise<RoutingDecision> {
  const { taskText, workflows, history } = input;

  const workflowCandidate = workflows
    .map((workflow) => ({
      workflow,
      score: Math.max(
        overlapScore(taskText, workflow.description),
        ...workflow.examplePrompts.map((prompt) => overlapScore(taskText, prompt)),
      ),
    }))
    .sort((a, b) => b.score - a.score)[0];

  const historyCandidate = history
    .map((record) => ({ record, score: overlapScore(taskText, record.taskText) }))
    .sort((a, b) => b.score - a.score)[0];

  const bestScore = Math.max(workflowCandidate?.score ?? 0, historyCandidate?.score ?? 0);

  if (bestScore >= 0.6 && workflowCandidate?.workflow.successRate >= 0.7) {
    return {
      route: "workflow",
      repeatable: true,
      confidence: "high",
      reason: `Matched reusable workflow ${workflowCandidate.workflow.workflowId} with score ${bestScore.toFixed(2)}`,
      workflowId: workflowCandidate.workflow.workflowId,
    };
  }

  if (bestScore >= 0.6 && !workflowCandidate?.workflow.workflowId && historyCandidate?.record.success) {
    return {
      route: "generated_workflow",
      repeatable: true,
      confidence: "high",
      reason: `Detected a repeatable task from history with score ${bestScore.toFixed(2)} but no reusable workflow asset exists yet`,
    };
  }

  if (bestScore >= 0.35) {
    return {
      route: "clarification",
      repeatable: true,
      confidence: "medium",
      reason: "Found a partially similar task but missing enough structure to route safely",
      clarificationQuestion: "I found a partially similar repeatable task, but I am missing the required inputs. What specific source, target, or date range should I use?",
    };
  }

  const oneOffKeywords = ["analyze", "investigate", "review", "look through", "why", "unusual", "总结", "分析", "排查"];
  if (oneOffKeywords.some((keyword) => taskText.toLowerCase().includes(keyword.toLowerCase()))) {
    return {
      route: "agent",
      repeatable: false,
      confidence: "high",
      reason: "Detected exploratory one-off language that should stay on the agent path",
    };
  }

  return {
    route: "clarification",
    repeatable: false,
    confidence: "low",
    reason: "No strong repeatable pattern and not enough context to pick the agent path confidently",
    clarificationQuestion: "I am missing the concrete inputs for this task. What system, files, or date range should this run against?",
  };
}
```

- [ ] **Step 4: Run the router test to verify it passes**

Run: `cd agent-service; npm run build; node dist/task_router/router.test.js`

Expected: PASS with `task-router tests passed`

- [ ] **Step 5: Commit or checkpoint**

```powershell
if (Test-Path .git) {
  git add agent-service/src/task_router/types.ts agent-service/src/task_router/scorer.ts agent-service/src/task_router/router.ts agent-service/src/task_router/router.test.ts
  git commit -m "feat: add deterministic task router"
} else {
  Write-Host "No git root; checkpoint feat: add deterministic task router"
}
```

### Task 2: Add local task history and workflow asset metadata stores

**Files:**
- Create: `agent-service/src/task_router/history_store.ts`
- Create: `agent-service/src/task_router/workflow_metadata_store.ts`
- Modify: `agent-service/src/task_router/types.ts`
- Test: `agent-service/src/task_router/router.test.ts`

- [ ] **Step 1: Extend the router tests to cover persistence-backed metadata**

```ts
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { FileTaskHistoryStore } from "./history_store";
import { FileWorkflowMetadataStore } from "./workflow_metadata_store";

const dir = mkdtempSync(join(tmpdir(), "memflow-router-"));
const historyPath = join(dir, "history.json");
const metadataPath = join(dir, "workflow-metadata.json");

const historyStore = new FileTaskHistoryStore(historyPath);
await historyStore.append({
  taskText: "nightly export invoices",
  route: "workflow",
  workflowId: "wf_invoice_export",
  success: true,
  parameterKeys: ["account_id", "date_range"],
  outputType: "file",
  createdAt: "2026-04-30T00:00:00.000Z",
});

const metadataStore = new FileWorkflowMetadataStore(metadataPath);
await metadataStore.upsert({
  workflowId: "wf_invoice_export",
  description: "Export invoices for a date range",
  inputHints: ["account_id", "date_range"],
  outputType: "file",
  successRate: 1,
  reusable: true,
  examplePrompts: ["nightly export invoices"],
  failureCategories: [],
  updatedAt: "2026-04-30T00:00:00.000Z",
});

const savedHistory = await historyStore.list();
const savedMetadata = await metadataStore.list();
assert.equal(savedHistory.length, 1);
assert.equal(savedMetadata[0]?.workflowId, "wf_invoice_export");

rmSync(dir, { recursive: true, force: true });
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd agent-service; npm run build; node dist/task_router/router.test.js`

Expected: FAIL with missing store module errors.

- [ ] **Step 3: Implement file-backed stores and metadata helpers**

```ts
// agent-service/src/task_router/history_store.ts
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import type { HistoricalTaskRecord } from "./types";

async function readJsonFile<T>(filePath: string, fallback: T): Promise<T> {
  try {
    const raw = await readFile(filePath, "utf8");
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

export class FileTaskHistoryStore {
  constructor(private readonly filePath: string) {}

  async list(): Promise<HistoricalTaskRecord[]> {
    return readJsonFile(this.filePath, []);
  }

  async append(record: HistoricalTaskRecord): Promise<void> {
    const current = await this.list();
    await mkdir(dirname(this.filePath), { recursive: true });
    await writeFile(this.filePath, JSON.stringify([record, ...current].slice(0, 100), null, 2));
  }
}

// agent-service/src/task_router/workflow_metadata_store.ts
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import type { WorkflowAssetMetadata } from "./types";

export class FileWorkflowMetadataStore {
  constructor(private readonly filePath: string) {}

  async list(): Promise<WorkflowAssetMetadata[]> {
    try {
      return JSON.parse(await readFile(this.filePath, "utf8")) as WorkflowAssetMetadata[];
    } catch {
      return [];
    }
  }

  async upsert(next: WorkflowAssetMetadata): Promise<void> {
    const current = await this.list();
    const merged = [...current.filter((item) => item.workflowId !== next.workflowId), next].sort((a, b) =>
      a.workflowId.localeCompare(b.workflowId),
    );
    await mkdir(dirname(this.filePath), { recursive: true });
    await writeFile(this.filePath, JSON.stringify(merged, null, 2));
  }
}
```

- [ ] **Step 4: Run the router test again**

Run: `cd agent-service; npm run build; node dist/task_router/router.test.js`

Expected: PASS with both routing and persistence assertions succeeding.

- [ ] **Step 5: Commit or checkpoint**

```powershell
if (Test-Path .git) {
  git add agent-service/src/task_router/history_store.ts agent-service/src/task_router/workflow_metadata_store.ts agent-service/src/task_router/router.test.ts
  git commit -m "feat: persist task routing history and workflow metadata"
} else {
  Write-Host "No git root; checkpoint feat: persist task routing history and workflow metadata"
}
```

### Task 3: Add a unified `/tasks/execute` flow in `agent-service`

**Files:**
- Create: `agent-service/src/task_entry.ts`
- Create: `agent-service/src/task_entry.test.ts`
- Modify: `agent-service/src/index.ts`
- Modify: `agent-service/src/messaging/nlp_workflow_creator.ts`
- Test: `agent-service/src/index.test.ts`

- [ ] **Step 1: Write failing tests for the unified task entry**

```ts
import assert from "node:assert/strict";
import { executeTaskEntry } from "./task_entry";

async function main() {
  const workflowResult = await executeTaskEntry(
    { text: "check pending orders and notify customers" },
    {
      decideRoute: async () => ({
        route: "workflow",
        repeatable: true,
        confidence: "high",
        reason: "Matched wf_daily_orders",
        workflowId: "wf_daily_orders",
      }),
      executeWorkflow: async () => ({ ok: true, summary: "workflow executed" }),
      generateWorkflow: async () => ({ workflowId: "wf_new_daily" }),
      executeAgent: async () => ({ content: "agent answer" }),
      appendHistory: async () => undefined,
      saveWorkflowMetadata: async () => undefined,
    },
  );
  assert.equal(workflowResult.route, "workflow");
  assert.equal(workflowResult.success, true);

  const generatedResult = await executeTaskEntry(
    { text: "create a repeatable nightly invoice export" },
    {
      decideRoute: async () => ({
        route: "generated_workflow",
        repeatable: true,
        confidence: "high",
        reason: "No asset exists yet",
      }),
      executeWorkflow: async (workflowId) => ({ ok: true, workflowId }),
      generateWorkflow: async () => ({ workflowId: "wf_invoice_export" }),
      executeAgent: async () => ({ content: "agent answer" }),
      appendHistory: async () => undefined,
      saveWorkflowMetadata: async () => undefined,
    },
  );
  assert.equal(generatedResult.route, "generated_workflow");
  assert.equal(generatedResult.workflow.workflowId, "wf_invoice_export");

  const clarificationResult = await executeTaskEntry(
    { text: "run the monthly export" },
    {
      decideRoute: async () => ({
        route: "clarification",
        repeatable: false,
        confidence: "medium",
        reason: "Missing required target information",
        clarificationQuestion: "Which account and date range should I use?",
      }),
      executeWorkflow: async () => ({ ok: true }),
      generateWorkflow: async () => ({ workflowId: "unused" }),
      executeAgent: async () => ({ content: "unused" }),
      appendHistory: async () => undefined,
      saveWorkflowMetadata: async () => undefined,
    },
  );
  assert.equal(clarificationResult.success, false);
  assert.equal(clarificationResult.clarificationQuestion, "Which account and date range should I use?");

  console.log("task-entry tests passed");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
```

- [ ] **Step 2: Run the task-entry test to verify it fails**

Run: `cd agent-service; npm run build; node dist/task_entry.test.js`

Expected: FAIL with `Cannot find module './task_entry'`.

- [ ] **Step 3: Implement the unified task execution service and wire it into Express**

```ts
// agent-service/src/task_entry.ts
import type { RoutingDecision, WorkflowAssetMetadata } from "./task_router/types";

export interface TaskExecutionResponse {
  route: RoutingDecision["route"];
  repeatable: boolean;
  confidence: RoutingDecision["confidence"];
  reason: string;
  success: boolean;
  clarificationQuestion?: string;
  workflow: { workflowId: string; generated: boolean } | null;
  result: unknown;
  failureCategory?: string;
}

export async function executeTaskEntry(
  input: { text: string },
  deps: {
    decideRoute: (input: { text: string }) => Promise<RoutingDecision>;
    executeWorkflow: (workflowId: string, params?: Record<string, unknown>) => Promise<unknown>;
    generateWorkflow: (text: string) => Promise<{ workflowId: string }>;
    executeAgent: (text: string) => Promise<{ content: string }>;
    appendHistory: (record: {
      taskText: string;
      route: "workflow" | "generated_workflow" | "agent";
      workflowId?: string;
      success: boolean;
      parameterKeys: string[];
      outputType: string;
      createdAt: string;
    }) => Promise<void>;
    saveWorkflowMetadata: (metadata: WorkflowAssetMetadata) => Promise<void>;
  },
): Promise<TaskExecutionResponse> {
  const decision = await deps.decideRoute({ text: input.text });

  if (decision.route === "clarification") {
    return {
      route: decision.route,
      repeatable: decision.repeatable,
      confidence: decision.confidence,
      reason: decision.reason,
      success: false,
      clarificationQuestion: decision.clarificationQuestion,
      workflow: null,
      result: null,
      failureCategory: "missing_parameters",
    };
  }

  if (decision.route === "agent") {
    const agentResult = await deps.executeAgent(input.text);
    await deps.appendHistory({
      taskText: input.text,
      route: "agent",
      success: true,
      parameterKeys: [],
      outputType: "text",
      createdAt: new Date().toISOString(),
    });
    return {
      route: "agent",
      repeatable: false,
      confidence: decision.confidence,
      reason: decision.reason,
      success: true,
      workflow: null,
      result: agentResult,
    };
  }

  const workflowId =
    decision.route === "generated_workflow"
      ? (await deps.generateWorkflow(input.text)).workflowId
      : decision.workflowId!;
  const result = await deps.executeWorkflow(workflowId);
  if (decision.route === "generated_workflow") {
    await deps.saveWorkflowMetadata({
      workflowId,
      description: input.text,
      inputHints: [],
      outputType: "structured",
      successRate: 1,
      reusable: true,
      examplePrompts: [input.text],
      failureCategories: [],
      updatedAt: new Date().toISOString(),
    });
  }
  await deps.appendHistory({
    taskText: input.text,
    route: decision.route,
    workflowId,
    success: true,
    parameterKeys: [],
    outputType: "structured",
    createdAt: new Date().toISOString(),
  });

  return {
    route: decision.route,
    repeatable: true,
    confidence: decision.confidence,
    reason: decision.reason,
    success: true,
    workflow: { workflowId, generated: decision.route === "generated_workflow" },
    result,
  };
}

// agent-service/src/index.ts
const historyStore = new FileTaskHistoryStore(process.env.MEMFLOW_TASK_HISTORY_PATH || "./.memflow-runtime/config/task-history.json");
const metadataStore = new FileWorkflowMetadataStore(process.env.MEMFLOW_WORKFLOW_METADATA_PATH || "./.memflow-runtime/config/workflow-metadata.json");

app.post("/tasks/execute", async (req, res) => {
  const { text } = req.body as { text: string };
  if (!text?.trim()) {
    res.status(400).json({ error: "text is required" });
    return;
  }

  const response = await executeTaskEntry(
    { text },
    {
      decideRoute: async ({ text }) =>
        decideTaskRoute({
          taskText: text,
          workflows: await metadataStore.list(),
          history: await historyStore.list(),
        }),
      executeWorkflow: async (workflowId, params) => executeWorkflow(workflowId, params as Record<string, unknown>),
      generateWorkflow: async (text) => workflowCreator.createFromNaturalLanguage("web", "local-user", text),
      executeAgent: async (text) => {
        const settings = await getLLMSettings();
        const completion = await createChatCompletion(settings, [
          { role: "system", content: "You are the one-off task path for MemFlow. Answer directly and explain concise next steps." },
          { role: "user", content: text },
        ]);
        return { content: completion.content };
      },
      appendHistory: async (record) => historyStore.append(record),
      saveWorkflowMetadata: async (metadata) => metadataStore.upsert(metadata),
    },
  );

  res.json(response);
});

app.get("/tasks/history", async (_req, res) => {
  res.json({ items: await historyStore.list() });
});

// agent-service/src/messaging/nlp_workflow_creator.ts
const response = await axios.post(
  `${EXECUTOR_URL}/execute`,
  {
    workflow_id: workflowId,
    params: params || {},
  },
  {
    headers: {
      "Content-Type": "application/json",
      "X-API-Key": EXECUTOR_API_KEY,
    },
  },
);
```

- [ ] **Step 4: Run the service self-tests**

Run: `cd agent-service; npm test`

Expected: PASS with both `agent-service self-test passed` and `task-entry tests passed`

- [ ] **Step 5: Commit or checkpoint**

```powershell
if (Test-Path .git) {
  git add agent-service/src/task_entry.ts agent-service/src/task_entry.test.ts agent-service/src/index.ts agent-service/src/index.test.ts agent-service/src/messaging/nlp_workflow_creator.ts
  git commit -m "feat: add unified task execution entrypoint"
} else {
  Write-Host "No git root; checkpoint feat: add unified task execution entrypoint"
}
```

### Task 4: Add the task-first frontend console and client API

**Files:**
- Create: `web-ui/src/components/TaskConsole.tsx`
- Create: `web-ui/src/components/task-console/TaskRouteCard.tsx`
- Create: `web-ui/src/components/task-console/TaskExecutionTimeline.tsx`
- Create: `web-ui/src/components/task-console/TaskResultPanel.tsx`
- Modify: `web-ui/src/api/client.ts`
- Modify: `web-ui/src/App.tsx`
- Modify: `web-ui/src/components/Layout.tsx`
- Modify: `web-ui/package.json`
- Test: `web-ui/src/components/__tests__/TaskConsole.test.tsx`

- [ ] **Step 1: Add a failing UI test for the task console**

```tsx
import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { test, expect, vi } from "vitest";
import TaskConsole from "../TaskConsole";

vi.mock("../../api/client", () => ({
  taskApi: {
    execute: vi.fn(),
    history: vi.fn(),
  },
}));

test("renders task input and routing explanation area", () => {
  render(
    <MemoryRouter>
      <TaskConsole />
    </MemoryRouter>,
  );

  expect(screen.getByPlaceholderText(/describe a task|描述一个任务/i)).toBeInTheDocument();
  expect(screen.getByText(/Routing decision|路由判断/i)).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the UI test to verify it fails**

Run: `cd web-ui; npm run test -- --run src/components/__tests__/TaskConsole.test.tsx`

Expected: FAIL because the test script and `TaskConsole` do not exist yet.

- [ ] **Step 3: Add the task console components, route, and test tooling**

```json
// web-ui/package.json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "lint": "eslint .",
    "preview": "vite preview",
    "test": "vitest --environment jsdom"
  },
  "devDependencies": {
    "@testing-library/jest-dom": "^6.6.3",
    "@testing-library/react": "^16.0.1",
    "jsdom": "^25.0.1",
    "vitest": "^2.1.3"
  }
}
```

```ts
// web-ui/src/api/client.ts
export interface TaskExecutionResponse {
  route: "workflow" | "generated_workflow" | "agent" | "clarification";
  repeatable: boolean;
  confidence: "high" | "medium" | "low";
  reason: string;
  success: boolean;
  clarificationQuestion?: string;
  workflow: { workflowId: string; generated: boolean } | null;
  result: unknown;
  failureCategory?: string;
}

export const taskApi = {
  execute: async (text: string): Promise<TaskExecutionResponse> => {
    const response = await api.post("/tasks/execute", { text });
    return response.data;
  },
  history: async (): Promise<any[]> => {
    const response = await api.get("/tasks/history");
    return response.data.items || [];
  },
};
```

```tsx
// web-ui/src/components/TaskConsole.tsx
import { useState } from "react";
import { taskApi, type TaskExecutionResponse } from "../api/client";
import TaskRouteCard from "./task-console/TaskRouteCard";
import TaskExecutionTimeline from "./task-console/TaskExecutionTimeline";
import TaskResultPanel from "./task-console/TaskResultPanel";

export default function TaskConsole() {
  const [text, setText] = useState("");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<TaskExecutionResponse | null>(null);

  async function submitTask() {
    setLoading(true);
    try {
      setResult(await taskApi.execute(text));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="mx-auto flex h-full w-full max-w-6xl flex-col gap-6 px-6 py-6">
      <section className="rounded-3xl border border-white/10 bg-slate-950/70 p-6">
        <h1 className="text-2xl font-semibold text-white">Task Console</h1>
        <p className="mt-2 text-sm text-slate-400">Describe a task and let MemFlow decide whether it should run through a workflow or the agent path.</p>
        <textarea
          value={text}
          onChange={(event) => setText(event.target.value)}
          placeholder="Describe a task or automation request"
          className="mt-4 min-h-40 w-full rounded-2xl border border-white/10 bg-slate-900 p-4 text-sm text-white"
        />
        <button onClick={submitTask} disabled={loading || !text.trim()} className="mt-4 rounded-2xl bg-cyan-400 px-4 py-2 text-sm font-medium text-slate-950 disabled:opacity-50">
          {loading ? "Running..." : "Run task"}
        </button>
      </section>

      <TaskRouteCard result={result} />
      <TaskExecutionTimeline result={result} />
      <TaskResultPanel result={result} />
    </div>
  );
}

// web-ui/src/components/task-console/TaskRouteCard.tsx
import type { TaskExecutionResponse } from "../../api/client";

export default function TaskRouteCard({ result }: { result: TaskExecutionResponse | null }) {
  return (
    <section className="rounded-3xl border border-white/10 bg-slate-950/70 p-6">
      <div className="text-sm font-medium text-white">Routing decision</div>
      <div className="mt-2 text-sm text-slate-300">{result ? result.reason : "The route explanation will appear here after execution."}</div>
    </section>
  );
}

// web-ui/src/components/task-console/TaskExecutionTimeline.tsx
import type { TaskExecutionResponse } from "../../api/client";

export default function TaskExecutionTimeline({ result }: { result: TaskExecutionResponse | null }) {
  const steps = result
    ? ["Task received", `Route selected: ${result.route}`, result.success ? "Execution succeeded" : "Execution needs attention"]
    : ["Task received", "Route selected", "Execution finished"];
  return (
    <section className="rounded-3xl border border-white/10 bg-slate-950/70 p-6">
      <div className="text-sm font-medium text-white">Execution timeline</div>
      <ol className="mt-3 space-y-2 text-sm text-slate-300">
        {steps.map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ol>
    </section>
  );
}

// web-ui/src/components/task-console/TaskResultPanel.tsx
import type { TaskExecutionResponse } from "../../api/client";

export default function TaskResultPanel({ result }: { result: TaskExecutionResponse | null }) {
  if (!result) {
    return (
      <section className="rounded-3xl border border-white/10 bg-slate-950/70 p-6 text-sm text-slate-400">
        Result details and recovery guidance will appear here.
      </section>
    );
  }

  return (
    <section className="rounded-3xl border border-white/10 bg-slate-950/70 p-6">
      <div className="text-sm font-medium text-white">Result</div>
      <pre className="mt-3 overflow-x-auto rounded-2xl bg-slate-900 p-4 text-xs text-slate-200">{JSON.stringify(result, null, 2)}</pre>
    </section>
  );
}

// web-ui/src/App.tsx
<Routes>
  <Route path="/" element={<TaskConsole />} />
  <Route path="/tasks" element={<TaskConsole />} />
  <Route path="/dashboard" element={<Dashboard />} />
  <Route path="/create" element={<NLCreator />} />
  <Route path="/settings" element={<Settings />} />
  <Route path="/editor" element={<WorkflowEditor />} />
</Routes>

// web-ui/src/components/Layout.tsx
const navItems = [
  { to: "/tasks", title: text({ zh: "任务控制台", en: "Task Console" }) },
  { to: "/create", title: text({ zh: "工作流生成", en: "Workflow Builder" }) },
  { to: "/dashboard", title: text({ zh: "概览", en: "Overview" }) },
  { to: "/settings", title: text({ zh: "设置", en: "Settings" }) },
];
```

- [ ] **Step 4: Run build and UI tests**

Run: `cd web-ui; npm run test -- --run src/components/__tests__/TaskConsole.test.tsx; npm run build`

Expected: PASS for the Vitest spec and a successful Vite production build.

- [ ] **Step 5: Commit or checkpoint**

```powershell
if (Test-Path .git) {
  git add web-ui/package.json web-ui/src/api/client.ts web-ui/src/App.tsx web-ui/src/components/TaskConsole.tsx web-ui/src/components/task-console/TaskRouteCard.tsx web-ui/src/components/task-console/TaskExecutionTimeline.tsx web-ui/src/components/task-console/TaskResultPanel.tsx web-ui/src/components/__tests__/TaskConsole.test.tsx web-ui/src/components/Layout.tsx
  git commit -m "feat: add task-first frontend console"
} else {
  Write-Host "No git root; checkpoint feat: add task-first frontend console"
}
```

### Task 5: Reshape workflow assets, execution history, and navigation

**Files:**
- Create: `web-ui/src/components/WorkflowAssetsPage.tsx`
- Create: `web-ui/src/components/TaskHistoryPage.tsx`
- Modify: `web-ui/src/components/Dashboard.tsx`
- Modify: `web-ui/src/components/ExecutionLogs.tsx`
- Modify: `web-ui/src/components/NLCreator.tsx`
- Modify: `web-ui/src/components/Layout.tsx`
- Test: `web-ui/src/components/__tests__/TaskConsole.test.tsx`

- [ ] **Step 1: Add failing assertions for asset and history navigation**

```tsx
import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { test, expect } from "vitest";
import Layout from "../Layout";

test("shows task-first navigation labels", async () => {
  render(
    <MemoryRouter>
      <Layout>
        <div>child</div>
      </Layout>
    </MemoryRouter>,
  );

  expect(screen.getByText(/Task Console|任务控制台/i)).toBeInTheDocument();
  expect(screen.getByText(/Workflow Assets|工作流资产/i)).toBeInTheDocument();
  expect(screen.getByText(/Execution History|执行记录/i)).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the UI test to verify it fails**

Run: `cd web-ui; npm run test -- --run src/components/__tests__/TaskConsole.test.tsx`

Expected: FAIL because `Layout` still exposes the old first-level navigation labels.

- [ ] **Step 3: Implement the supporting pages and demote old primary screens**

```tsx
// web-ui/src/components/WorkflowAssetsPage.tsx
import { useEffect, useState } from "react";
import { workflowApi } from "../api/client";

export default function WorkflowAssetsPage() {
  const [items, setItems] = useState<any[]>([]);

  useEffect(() => {
    void workflowApi.listWorkflows().then((payload: any) => setItems(Array.isArray(payload) ? payload : payload.workflows ?? []));
  }, []);

  return (
    <div className="mx-auto max-w-6xl px-6 py-6">
      <h1 className="text-2xl font-semibold text-white">Workflow Assets</h1>
      <p className="mt-2 text-sm text-slate-400">Review reusable workflow assets, generated flows, and their success signals.</p>
      <div className="mt-6 grid gap-4">
        {items.map((item) => (
          <article key={item.id} className="rounded-2xl border border-white/10 bg-slate-950/70 p-4">
            <div className="text-sm font-medium text-white">{item.name || item.id}</div>
            <div className="mt-1 text-xs text-slate-400">Workflow ID: {item.id}</div>
          </article>
        ))}
      </div>
    </div>
  );
}

// web-ui/src/components/TaskHistoryPage.tsx
import { useEffect, useState } from "react";
import { taskApi } from "../api/client";

export default function TaskHistoryPage() {
  const [items, setItems] = useState<any[]>([]);

  useEffect(() => {
    void taskApi.history().then(setItems);
  }, []);

  return (
    <div className="mx-auto max-w-6xl px-6 py-6">
      <h1 className="text-2xl font-semibold text-white">Execution History</h1>
      <div className="mt-6 grid gap-4">
        {items.map((item, index) => (
          <article key={`${item.taskText}-${index}`} className="rounded-2xl border border-white/10 bg-slate-950/70 p-4">
            <div className="text-sm font-medium text-white">{item.taskText}</div>
            <div className="mt-1 text-xs text-slate-400">Route: {item.route}</div>
          </article>
        ))}
      </div>
    </div>
  );
}

// web-ui/src/components/Layout.tsx
const navItems = [
  { to: "/tasks", title: text({ zh: "任务控制台", en: "Task Console" }) },
  { to: "/assets", title: text({ zh: "工作流资产", en: "Workflow Assets" }) },
  { to: "/history", title: text({ zh: "执行记录", en: "Execution History" }) },
  { to: "/settings", title: text({ zh: "系统设置", en: "Settings" }) },
  { to: "/advanced", title: text({ zh: "高级能力", en: "Advanced" }) },
];

// web-ui/src/components/Dashboard.tsx
export default function Dashboard() {
  return (
    <div className="mx-auto max-w-6xl px-6 py-6">
      <h1 className="text-2xl font-semibold text-white">Advanced</h1>
      <p className="mt-2 text-sm text-slate-400">Keep timeline, marketplace, and autonomous controls here until Stage 2 work promotes them again.</p>
    </div>
  );
}

// web-ui/src/components/ExecutionLogs.tsx
export default function ExecutionLogs() {
  return <TaskHistoryPage />;
}

// web-ui/src/components/NLCreator.tsx
export default function NLCreator() {
  return (
    <div className="mx-auto max-w-6xl px-6 py-6">
      <TaskConsole />
    </div>
  );
}
```

- [ ] **Step 4: Run the UI test and production build**

Run: `cd web-ui; npm run test -- --run src/components/__tests__/TaskConsole.test.tsx; npm run build`

Expected: PASS with updated navigation labels and page routes.

- [ ] **Step 5: Commit or checkpoint**

```powershell
if (Test-Path .git) {
  git add web-ui/src/components/WorkflowAssetsPage.tsx web-ui/src/components/TaskHistoryPage.tsx web-ui/src/components/Dashboard.tsx web-ui/src/components/ExecutionLogs.tsx web-ui/src/components/NLCreator.tsx web-ui/src/components/Layout.tsx
  git commit -m "feat: reshape navigation around task console"
} else {
  Write-Host "No git root; checkpoint feat: reshape navigation around task console"
}
```

### Task 6: Clean duplicate source drift and document the Stage 1 product path

**Files:**
- Modify: `README.md`
- Delete: `web-ui/src/src/App.tsx`
- Delete: `web-ui/src/src/index.css`
- Delete: `web-ui/src/src/main.tsx`
- Delete: `web-ui/src/src/vite-env.d.ts`
- Delete: `web-ui/src/src/api/client.ts`
- Delete: `web-ui/src/src/components/CustomNode.tsx`
- Delete: `web-ui/src/src/components/DiffViewer.tsx`
- Delete: `web-ui/src/src/components/NodePalette.tsx`
- Delete: `web-ui/src/src/components/PropertyPanel.tsx`
- Delete: `web-ui/src/src/components/WorkflowEditor.tsx`
- Delete: `web-ui/src/src/stores/workflowStore.ts`

- [ ] **Step 1: Write a failing verification command for duplicate frontend source**

```powershell
$duplicateTree = Test-Path "web-ui\src\src"
if ($duplicateTree) { throw "Duplicate frontend source tree still exists" }
```

- [ ] **Step 2: Run the verification command to confirm it fails before cleanup**

Run: `powershell -NoProfile -Command "$duplicateTree = Test-Path 'web-ui\src\src'; if ($duplicateTree) { throw 'Duplicate frontend source tree still exists' }"`

Expected: FAIL with `Duplicate frontend source tree still exists`

- [ ] **Step 3: Remove duplicate sources and update README**

```md
<!-- README.md -->
## Stage 1 Product Path

MemFlow now runs through a workflow-first task console:

1. Submit a natural-language task
2. MemFlow decides whether the task is repeatable or one-off
3. Repeatable tasks route to an existing workflow or a generated workflow
4. One-off tasks route to the agent path
5. The UI explains the route, execution outcome, and recovery guidance

## Runtime Roles

- `executor/` remains the workflow execution kernel
- `agent-service/` is the primary task-routing entrypoint
- `web-ui/` is the user-facing task console
- `main.py` and `core/agent_loop.py` remain experimental
```

```powershell
Remove-Item -Recurse -Force web-ui\src\src
```

- [ ] **Step 4: Run the full verification sweep**

Run: `cd agent-service; npm test; cd ..\web-ui; npm run test -- --run src/components/__tests__/TaskConsole.test.tsx; npm run build; cd ..; powershell -NoProfile -Command "$duplicateTree = Test-Path 'web-ui\src\src'; if ($duplicateTree) { throw 'Duplicate frontend source tree still exists' } else { Write-Host 'duplicate tree removed' }"`

Expected:
- `agent-service self-test passed`
- `task-entry tests passed`
- Vitest passes
- Vite build succeeds
- Final PowerShell command prints `duplicate tree removed`

- [ ] **Step 5: Commit or checkpoint**

```powershell
if (Test-Path .git) {
  git add README.md web-ui/src web-ui/package.json
  git commit -m "chore: clean duplicate frontend sources and document stage 1 path"
} else {
  Write-Host "No git root; checkpoint chore: clean duplicate frontend sources and document stage 1 path"
}
```

## Self-Review

### 1. Spec coverage

- Task routing rule: covered by Task 1 and Task 3
- Repeatable vs one-off decision and clarification fallback: covered by Task 1 and Task 3
- Unified result model: covered by Task 3 and Task 4
- Workflow asset metadata and historical evidence: covered by Task 2
- Task-first console and route visibility: covered by Task 4
- Workflow assets, execution history, settings-first navigation: covered by Task 5
- Duplicate source cleanup and runtime-role documentation: covered by Task 6

No Stage 1 spec requirement is left without a task.

### 2. Placeholder scan

This plan avoids placeholder language and abstract implementation notes. Every task has explicit files, commands, and code snippets.

### 3. Type consistency

The plan consistently uses:

- `TaskExecutionResponse`
- `RoutingDecision`
- `WorkflowAssetMetadata`
- `HistoricalTaskRecord`
- `executeTaskEntry`

These names are introduced before later tasks depend on them.
