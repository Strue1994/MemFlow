import assert from "node:assert/strict";
import http from "node:http";
import { buildExecutorHeaders, loadAvailableWorkflows } from "./index";
import { buildRecoveryPlan, classifyFailure, dispatchAutoFix, executeRecoveryPlan, planVerification, runVerificationPlan } from "./coding_kernel";
import { createApp } from "./index";
import { NlpWorkflowCreator } from "./messaging/nlp_workflow_creator";
import { runTaskEntryTests } from "./task_entry.test";
import type { HistoricalTaskRecord, WorkflowAssetMetadata } from "./task_router/types";
import axios from "axios";

async function main() {
  const headers = buildExecutorHeaders(true);
  assert.equal(headers["Content-Type"], "application/json");

  const originalAxiosPost = axios.post;
  let capturedAxiosUrl = "";
  axios.post = (async (url: string) => {
    capturedAxiosUrl = url;
    return {
      data: {
        workflow_id: "wf_generated",
      },
    };
  }) as typeof axios.post;

  const originalFetch = globalThis.fetch;
  globalThis.fetch = (async () => {
    return new Response(
      JSON.stringify([
        { id: "wf_zen", name: "Zen workflow", version: 1 },
        { id: "wf_add", name: "Add workflow", version: 1 },
      ]),
      {
        status: 200,
        headers: {
          "Content-Type": "application/json",
        },
      },
    );
  }) as typeof fetch;

  try {
    const creator = new NlpWorkflowCreator();
    await creator.createFromNaturalLanguage("web", "local-user", "test generator default");
    assert.equal(capturedAxiosUrl, "http://127.0.0.1:8082/create_workflow_v2");

    const workflows = await loadAvailableWorkflows();
    assert.ok(workflows.some((workflow) => workflow.id === "wf_zen"));
    assert.ok(workflows.some((workflow) => workflow.id === "wf_add"));

    const classification = classifyFailure({ error: "Invalid return: no value to return" });
    assert.equal(classification.category, "invalid_return");
    const invalidUrl = classifyFailure({ error: "Failed to parse URL from http://127.0.0.1:8082 /workflows" });
    assert.equal(invalidUrl.category, "invalid_url");
    const executorUnavailable = classifyFailure({ error: "fetch failed while contacting http://127.0.0.1:8082/workflows" });
    assert.equal(executorUnavailable.category, "executor_unavailable");

    const verificationPlan = planVerification({
      taskType: "feature",
      objective: "Build coding kernel",
      changedFiles: ["agent-service/src/coding_kernel.ts", "agent-service/src/index.ts"],
    });
    assert.ok(verificationPlan.checks.length >= 2);
    assert.ok(verificationPlan.checks.some((check) => check.id === "agent-build"));
    assert.ok(verificationPlan.checks.some((check) => check.id === "git-status"));
    assert.ok(verificationPlan.checks.some((check) => check.id === "coding-kernel-artifact"));

    const recoveryPlan = buildRecoveryPlan({
      error: "Access denied (os error 5) while replacing executor.exe",
      changedFiles: ["executor/src/lib.rs"],
    });
    assert.equal(recoveryPlan.classification.category, "permission_denied");
    assert.ok(recoveryPlan.actions.length >= 1);

    const fileVerification = await runVerificationPlan({
      taskType: "feature",
      objective: "File verification smoke",
      plan: {
        taskType: "feature",
        objective: "File verification smoke",
        changedFiles: ["agent-service/src/coding_kernel.ts"],
        evidencePaths: [],
        checks: [
          {
            id: "current-test-file",
            label: "Current test file exists",
            kind: "file",
            filePath: "agent-service/src/index.test.ts",
            fileRoot: "project",
            mustExist: true,
            containsText: "agent-service self-test passed",
            reason: "Smoke test the file verifier against a known source file.",
          },
        ],
      },
    });
    assert.equal(fileVerification.success, true);

    const recoveryExecution = await executeRecoveryPlan({
      error: "Address already in use: 3000",
      changedFiles: ["agent-service/src/index.ts"],
    });
    assert.equal(recoveryExecution.classification.category, "port_conflict");
    assert.ok(recoveryExecution.actionResults.length >= 2);

    const dispatchReport = await dispatchAutoFix({
      error: "Access denied (os error 5) while replacing executor.exe",
      changedFiles: ["scripts/dev-local.ps1"],
    });
    assert.equal(dispatchReport.classification.category, "permission_denied");
    assert.ok(dispatchReport.dispatchResults.length >= 1);

    const urlDispatch = await dispatchAutoFix({
      error: "Failed to parse URL from http://127.0.0.1:8082 /workflows",
      changedFiles: ["agent-service/src/index.ts", "agent-service/src/autonomy_supervisor.ts"],
    });
    assert.equal(urlDispatch.classification.category, "invalid_url");
    assert.ok(urlDispatch.dispatchResults.length >= 1);

    const executorDispatch = await dispatchAutoFix({
      error: "fetch failed while contacting http://127.0.0.1:8082/workflows",
      changedFiles: ["scripts/restart-live-executor.ps1"],
    });
    assert.equal(executorDispatch.classification.category, "executor_unavailable");
    assert.ok(executorDispatch.dispatchResults.length >= 1);

    globalThis.fetch = originalFetch;
    await runTaskEntryTests();

    const historyItems: HistoricalTaskRecord[] = [];
    const workflowItems: WorkflowAssetMetadata[] = [
      {
        workflowId: "wf_existing",
        description: "generate daily orders report",
        inputHints: [],
        outputType: "report",
        successRate: 0.9,
        reusable: true,
        examplePrompts: ["generate daily orders report"],
        failureCategories: [],
        updatedAt: "2026-04-30T00:00:00.000Z",
      },
    ];

    const app = createApp({
      historyStore: {
        list: async () => historyItems,
        append: async (record: HistoricalTaskRecord) => {
          historyItems.unshift(record);
        },
      },
      workflowMetadataStore: {
        list: async () => workflowItems,
        upsert: async (record: WorkflowAssetMetadata) => {
          workflowItems.push(record);
        },
      },
      executeWorkflow: async (workflowId: string, params?: Record<string, unknown>) => ({
        workflowId,
        params: params || {},
        status: "ok",
      }),
      executeAgent: async (text: string) => ({
        content: `agent:${text}`,
      }),
      generateWorkflow: async () => ({
        workflowId: "wf_generated",
      }),
    });

    const server = http.createServer(app);
    await new Promise<void>((resolve) => server.listen(0, resolve));

    try {
      const address = server.address();
      assert.ok(address && typeof address === "object");
      const baseUrl = `http://127.0.0.1:${address.port}`;

      const workflowResponse = await fetch(`${baseUrl}/tasks/execute`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ text: "generate daily orders report", params: { date: "2026-04-30" } }),
      });
      assert.equal(workflowResponse.status, 200);
      const workflowBody = await workflowResponse.json() as {
        route: string;
        success: boolean;
        workflow: { workflowId: string; generated: boolean } | null;
      };
      assert.equal(workflowBody.route, "workflow");
      assert.equal(workflowBody.success, true);
      assert.equal(workflowBody.workflow?.workflowId, "wf_existing");

      const clarificationResponse = await fetch(`${baseUrl}/tasks/execute`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ text: "daily orders report for store" }),
      });
      assert.equal(clarificationResponse.status, 200);
      const clarificationBody = await clarificationResponse.json() as {
        route: string;
        success: boolean;
        workflow: { workflowId: string; generated: boolean } | null;
        failureCategory?: string;
        clarificationQuestion?: string;
      };
      assert.equal(clarificationBody.route, "clarification");
      assert.equal(clarificationBody.success, false);
      assert.equal(clarificationBody.workflow, null);
      assert.equal(clarificationBody.failureCategory, "missing_parameters");
      assert.match(clarificationBody.clarificationQuestion ?? "", /missing/i);

      historyItems.unshift({
        taskText: "prepare weekly supplier status digest",
        route: "generated_workflow",
        success: true,
        parameterKeys: ["week"],
        outputType: "digest",
        createdAt: "2026-04-30T00:00:00.000Z",
      });
      const generatedResponse = await fetch(`${baseUrl}/tasks/execute`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ text: "prepare weekly supplier status digest", params: { week: "2026-W18" } }),
      });
      assert.equal(generatedResponse.status, 200);
      const generatedBody = await generatedResponse.json() as {
        route: string;
        success: boolean;
        workflow: { workflowId: string; generated: boolean } | null;
      };
      assert.equal(generatedBody.route, "generated_workflow");
      assert.equal(generatedBody.success, true);
      assert.equal(generatedBody.workflow?.workflowId, "wf_generated");
      assert.equal(generatedBody.workflow?.generated, true);

      const agentResponse = await fetch(`${baseUrl}/tasks/execute`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ text: "investigate why the checkout error is unusual" }),
      });
      assert.equal(agentResponse.status, 200);
      const agentBody = await agentResponse.json() as {
        route: string;
        success: boolean;
        workflow: { workflowId: string; generated: boolean } | null;
        result: { content: string };
      };
      assert.equal(agentBody.route, "agent");
      assert.equal(agentBody.success, true);
      assert.equal(agentBody.workflow, null);
      assert.equal(agentBody.result.content, "agent:investigate why the checkout error is unusual");

      const historyResponse = await fetch(`${baseUrl}/tasks/history`);
      assert.equal(historyResponse.status, 200);
      const historyBody = await historyResponse.json() as { items: HistoricalTaskRecord[] };
      assert.ok(historyBody.items.length >= 3);
      assert.ok(historyBody.items.some((item) => item.route === "workflow"));
      assert.ok(historyBody.items.some((item) => item.route === "generated_workflow"));
      assert.ok(historyBody.items.some((item) => item.route === "agent"));
    } finally {
      await new Promise<void>((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
    }
  } finally {
    axios.post = originalAxiosPost;
    globalThis.fetch = originalFetch;
  }

  console.log("agent-service self-test passed");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
