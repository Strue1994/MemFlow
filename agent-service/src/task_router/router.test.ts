import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { FileTaskHistoryStore } from "./history_store";
import { decideTaskRoute } from "./router";
import { tokenize } from "./scorer";
import { FileWorkflowMetadataStore } from "./workflow_metadata_store";
import type { HistoricalTaskRecord, WorkflowAssetMetadata } from "./types";

function buildWorkflow(overrides: Partial<WorkflowAssetMetadata>): WorkflowAssetMetadata {
  return {
    workflowId: "wf_default",
    description: "",
    inputHints: [],
    outputType: "report",
    successRate: 0.8,
    reusable: true,
    examplePrompts: [],
    failureCategories: [],
    updatedAt: "2026-04-30T00:00:00.000Z",
    ...overrides,
  };
}

function buildHistory(overrides: Partial<HistoricalTaskRecord>): HistoricalTaskRecord {
  return {
    taskText: "",
    route: "workflow",
    success: true,
    parameterKeys: [],
    outputType: "report",
    createdAt: "2026-04-30T00:00:00.000Z",
    ...overrides,
  };
}

async function main() {
  assert.deepEqual(tokenize("daily_orders"), ["daily_orders"]);

  const tempDir = await mkdtemp(path.join(tmpdir(), "task-router-"));

  try {
    const historyStore = new FileTaskHistoryStore(
      path.join(tempDir, "history", "task-history.json"),
    );
    await historyStore.append(
      buildHistory({
        taskText: "export invoice records for march",
        workflowId: "wf_invoice_export",
      }),
    );

    const metadataStore = new FileWorkflowMetadataStore(
      path.join(tempDir, "metadata", "workflow-assets.json"),
    );
    await metadataStore.upsert(
      buildWorkflow({
        workflowId: "wf_invoice_export",
        description: "export invoice records",
        examplePrompts: ["export invoice records for march"],
      }),
    );

    const persistedHistory = await historyStore.list();
    const persistedMetadata = await metadataStore.list();
    assert.equal(persistedHistory.length, 1);
    assert.equal(persistedMetadata[0]?.workflowId, "wf_invoice_export");

    await Promise.all([
      historyStore.append(
        buildHistory({
          taskText: "export invoice records for april",
          workflowId: "wf_invoice_export",
          createdAt: "2026-04-30T00:00:01.000Z",
        }),
      ),
      historyStore.append(
        buildHistory({
          taskText: "export invoice records for may",
          workflowId: "wf_invoice_export",
          createdAt: "2026-04-30T00:00:02.000Z",
        }),
      ),
    ]);
    const concurrentHistory = await historyStore.list();
    assert.equal(concurrentHistory.length, 3);
    assert.equal(concurrentHistory[0]?.taskText, "export invoice records for may");

    await Promise.all([
      metadataStore.upsert(
        buildWorkflow({
          workflowId: "wf_billing_export",
          description: "export billing records",
        }),
      ),
      metadataStore.upsert(
        buildWorkflow({
          workflowId: "wf_shipping_export",
          description: "export shipping records",
        }),
      ),
    ]);
    const concurrentMetadata = await metadataStore.list();
    assert.deepEqual(
      concurrentMetadata.map((record) => record.workflowId),
      ["wf_billing_export", "wf_invoice_export", "wf_shipping_export"],
    );

    const workflows: WorkflowAssetMetadata[] = [
      buildWorkflow({
        workflowId: "wf_daily_orders",
        description: "generate daily orders report for yesterday",
        inputHints: ["store", "date range"],
        examplePrompts: ["generate daily orders report for yesterday"],
      }),
      buildWorkflow({
        workflowId: "wf_inventory_summary",
        description: "summarize current inventory exceptions",
        inputHints: ["warehouse"],
        examplePrompts: ["summarize current inventory exceptions for warehouse a"],
      }),
    ];

    const workflowDecision = decideTaskRoute({
      taskText: "generate daily orders report for yesterday",
      workflows,
      history: [],
    });
    assert.equal(workflowDecision.route, "workflow");
    assert.equal(workflowDecision.workflowId, "wf_daily_orders");
    assert.equal(workflowDecision.repeatable, true);
    assert.equal(workflowDecision.confidence, "high");

    const nonReusableDecision = decideTaskRoute({
      taskText: "generate monthly revenue report",
      workflows: [
        buildWorkflow({
          workflowId: "wf_monthly_revenue",
          description: "generate monthly revenue report",
          examplePrompts: ["generate monthly revenue report"],
          reusable: false,
        }),
      ],
      history: [],
    });
    assert.notEqual(nonReusableDecision.route, "workflow");

    const exploratoryDecision = decideTaskRoute({
      taskText: "investigate why the checkout error is unusual",
      workflows,
      history: [],
    });
    assert.equal(exploratoryDecision.route, "agent");
    assert.equal(exploratoryDecision.repeatable, false);
    assert.equal(exploratoryDecision.confidence, "high");

    const historyOnlyDecision = decideTaskRoute({
      taskText: "prepare weekly supplier status digest",
      workflows: [],
      history: [
        buildHistory({
          taskText: "prepare weekly supplier status digest",
          route: "generated_workflow",
          success: true,
          workflowId: "wf_generated_supplier_digest",
          parameterKeys: ["supplier", "week"],
          outputType: "digest",
        }),
      ],
    });
    assert.equal(historyOnlyDecision.route, "generated_workflow");
    assert.equal(historyOnlyDecision.repeatable, true);
    assert.equal(historyOnlyDecision.confidence, "high");
    assert.equal(historyOnlyDecision.workflowId, undefined);

    const historyWithoutWorkflowIdDecision = decideTaskRoute({
      taskText: "prepare recurring supplier status digest",
      workflows: [],
      history: [
        buildHistory({
          taskText: "prepare recurring supplier status digest",
          route: "generated_workflow",
          success: true,
        }),
      ],
    });
    assert.equal(historyWithoutWorkflowIdDecision.route, "generated_workflow");
    assert.equal(historyWithoutWorkflowIdDecision.workflowId, undefined);

    const clarificationDecision = decideTaskRoute({
      taskText: "daily orders report for store",
      workflows,
      history: [],
    });
    assert.equal(clarificationDecision.route, "clarification");
    assert.equal(clarificationDecision.repeatable, true);
    assert.equal(clarificationDecision.confidence, "medium");
    assert.match(clarificationDecision.clarificationQuestion ?? "", /missing/i);
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }

  console.log("task-router tests passed");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
