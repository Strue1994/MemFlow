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

export interface TaskEntryInput {
  text: string;
  params?: Record<string, unknown>;
  routeDecision: RoutingDecision;
  version?: number;
  timeoutSeconds?: number;
}

interface TaskHistoryStoreLike {
  append(record: {
    taskText: string;
    route: RoutingDecision["route"];
    workflowId?: string;
    success: boolean;
    parameterKeys: string[];
    outputType: string;
    createdAt: string;
  }): Promise<void>;
}

interface WorkflowMetadataStoreLike {
  upsert(record: WorkflowAssetMetadata): Promise<void>;
}

export interface TaskEntryDeps {
  executeWorkflow: (
    workflowId: string,
    params?: Record<string, unknown>,
    version?: number,
    timeoutSeconds?: number,
  ) => Promise<unknown>;
  executeAgent: (text: string) => Promise<unknown>;
  generateWorkflow: (text: string) => Promise<{ workflowId: string; message?: string }>;
  historyStore: TaskHistoryStoreLike;
  workflowMetadataStore: WorkflowMetadataStoreLike;
  now?: () => string;
}

function inferOutputType(result: unknown): string {
  if (result === null || result === undefined) {
    return "empty";
  }
  if (typeof result === "string") {
    return "text";
  }
  if (Array.isArray(result)) {
    return "list";
  }
  if (typeof result === "object") {
    return "json";
  }
  return typeof result;
}

async function appendHistory(
  deps: TaskEntryDeps,
  input: TaskEntryInput,
  route: RoutingDecision["route"],
  success: boolean,
  result: unknown,
  workflowId?: string,
): Promise<void> {
  await deps.historyStore.append({
    taskText: input.text,
    route,
    workflowId,
    success,
    parameterKeys: Object.keys(input.params || {}),
    outputType: inferOutputType(result),
    createdAt: (deps.now || (() => new Date().toISOString()))(),
  });
}

export async function executeTaskEntry(
  input: TaskEntryInput,
  deps: TaskEntryDeps,
): Promise<TaskExecutionResponse> {
  const { routeDecision } = input;

  if (routeDecision.route === "clarification") {
    return {
      route: routeDecision.route,
      repeatable: routeDecision.repeatable,
      confidence: routeDecision.confidence,
      reason: routeDecision.reason,
      success: false,
      clarificationQuestion: routeDecision.clarificationQuestion,
      workflow: null,
      result: null,
      failureCategory: "missing_parameters",
    };
  }

  if (routeDecision.route === "agent") {
    const result = await deps.executeAgent(input.text);
    await appendHistory(deps, input, "agent", true, result);
    return {
      route: routeDecision.route,
      repeatable: routeDecision.repeatable,
      confidence: routeDecision.confidence,
      reason: routeDecision.reason,
      success: true,
      workflow: null,
      result,
    };
  }

  if (routeDecision.route === "generated_workflow") {
    const generated = await deps.generateWorkflow(input.text);
    if (!generated.workflowId) {
      throw new Error("Workflow generator did not return a workflowId");
    }

    const result = await deps.executeWorkflow(
      generated.workflowId,
      input.params || {},
      input.version,
      input.timeoutSeconds,
    );

    await deps.workflowMetadataStore.upsert({
      workflowId: generated.workflowId,
      description: input.text,
      inputHints: Object.keys(input.params || {}),
      outputType: inferOutputType(result),
      successRate: 1,
      reusable: false,
      examplePrompts: [input.text],
      failureCategories: [],
      updatedAt: (deps.now || (() => new Date().toISOString()))(),
    });
    await appendHistory(deps, input, "generated_workflow", true, result, generated.workflowId);

    return {
      route: routeDecision.route,
      repeatable: routeDecision.repeatable,
      confidence: routeDecision.confidence,
      reason: routeDecision.reason,
      success: true,
      workflow: { workflowId: generated.workflowId, generated: true },
      result,
    };
  }

  if (!routeDecision.workflowId) {
    throw new Error("Workflow route requires a workflowId");
  }

  const result = await deps.executeWorkflow(
    routeDecision.workflowId,
    input.params || {},
    input.version,
    input.timeoutSeconds,
  );
  await appendHistory(deps, input, "workflow", true, result, routeDecision.workflowId);

  return {
    route: routeDecision.route,
    repeatable: routeDecision.repeatable,
    confidence: routeDecision.confidence,
    reason: routeDecision.reason,
    success: true,
    workflow: { workflowId: routeDecision.workflowId, generated: false },
    result,
  };
}
