import assert from "node:assert/strict";
import { executeTaskEntry } from "./task_entry";
import type { RoutingDecision, WorkflowAssetMetadata } from "./task_router/types";

function createDeps() {
  const history: Array<Record<string, unknown>> = [];
  const metadata: WorkflowAssetMetadata[] = [];

  return {
    history,
    metadata,
    deps: {
      executeWorkflow: async (workflowId: string, params?: Record<string, unknown>) => ({
        workflowId,
        params: params || {},
        ok: true,
      }),
      executeAgent: async (text: string) => ({ content: `agent:${text}` }),
      generateWorkflow: async (text: string) => ({
        workflowId: `generated:${text.replace(/\s+/g, "_")}`,
      }),
      historyStore: {
        append: async (record: Record<string, unknown>) => {
          history.push(record);
        },
      },
      workflowMetadataStore: {
        upsert: async (record: WorkflowAssetMetadata) => {
          metadata.push(record);
        },
      },
      now: () => "2026-04-30T00:00:00.000Z",
    },
  };
}

export async function runTaskEntryTests() {
  const workflowState = createDeps();
  const workflowDecision: RoutingDecision = {
    route: "workflow",
    repeatable: true,
    confidence: "high",
    reason: "matched reusable workflow",
    workflowId: "wf_existing",
  };
  const workflowResponse = await executeTaskEntry(
    {
      text: "run the existing workflow",
      params: { date: "2026-04-30" },
      routeDecision: workflowDecision,
    },
    workflowState.deps,
  );
  assert.equal(workflowResponse.route, "workflow");
  assert.equal(workflowResponse.success, true);
  assert.deepEqual(workflowResponse.workflow, {
    workflowId: "wf_existing",
    generated: false,
  });
  assert.equal(workflowState.history.length, 1);

  const generatedState = createDeps();
  const generatedDecision: RoutingDecision = {
    route: "generated_workflow",
    repeatable: true,
    confidence: "high",
    reason: "matched prior generated workflow pattern",
  };
  const generatedResponse = await executeTaskEntry(
    {
      text: "prepare weekly supplier digest",
      params: { week: "2026-W18" },
      routeDecision: generatedDecision,
    },
    generatedState.deps,
  );
  assert.equal(generatedResponse.route, "generated_workflow");
  assert.equal(generatedResponse.success, true);
  assert.equal(generatedResponse.workflow?.workflowId, "generated:prepare_weekly_supplier_digest");
  assert.equal(generatedResponse.workflow?.generated, true);
  assert.equal(generatedState.metadata[0]?.workflowId, "generated:prepare_weekly_supplier_digest");

  const clarificationState = createDeps();
  const clarificationDecision: RoutingDecision = {
    route: "clarification",
    repeatable: true,
    confidence: "medium",
    reason: "missing date range",
    clarificationQuestion: "Which date range should be used?",
  };
  const clarificationResponse = await executeTaskEntry(
    {
      text: "generate report",
      routeDecision: clarificationDecision,
    },
    clarificationState.deps,
  );
  assert.equal(clarificationResponse.success, false);
  assert.equal(clarificationResponse.failureCategory, "missing_parameters");
  assert.equal(clarificationResponse.clarificationQuestion, "Which date range should be used?");
  assert.equal(clarificationState.history.length, 0);

  console.log("task-entry tests passed");
}

async function main() {
  await runTaskEntryTests();
}

if (require.main === module) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
