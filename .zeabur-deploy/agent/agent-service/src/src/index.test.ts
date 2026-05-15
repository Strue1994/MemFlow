import assert from "node:assert/strict";
import { buildExecutorHeaders, loadAvailableWorkflows } from "./index";

async function main() {
  const headers = buildExecutorHeaders(true);
  assert.equal(headers["Content-Type"], "application/json");

  const originalFetch = globalThis.fetch;
  globalThis.fetch = (async () => {
    throw new Error("executor unavailable");
  }) as typeof fetch;

  try {
    const workflows = await loadAvailableWorkflows();
    assert.ok(workflows.some((workflow) => workflow.id === "wf_zen"));
    assert.ok(workflows.some((workflow) => workflow.id === "wf_add"));
  } finally {
    globalThis.fetch = originalFetch;
  }

  console.log("agent-service self-test passed");
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
