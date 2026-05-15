/**
 * Unit tests for core MemFlow modules
 * Tests: skill-loader, curator, context-compressor, checkpoints, middleware-chain
 */
import { describe, it, expect, beforeAll, afterAll } from "vitest";
import * as path from "node:path";
import * as fs from "node:fs";
import * as os from "node:os";

// ---- skill-loader tests ----
describe("skill-loader", () => {
  it("should parse SKILL.md frontmatter", async () => {
    const { loadSkillFile } = await import("../src/skill-loader");
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "skill-test-"));
    const skillDir = path.join(tmpDir, "test-skill");
    fs.mkdirSync(skillDir, { recursive: true });
    fs.writeFileSync(
      path.join(skillDir, "SKILL.md"),
      `---
name: test-skill
description: "Use when testing — validates SKILL.md parsing"
---
# Test Skill
Simple test`
    );
    const result = loadSkillFile(path.join(skillDir, "SKILL.md"));
    expect(result).not.toBeNull();
    expect(result!.name).toBe("test-skill");
    expect(result!.description).toContain("testing");
    fs.rmSync(tmpDir, { recursive: true });
  });

  it("should return null for invalid files", async () => {
    const { loadSkillFile } = await import("../src/skill-loader");
    const result = loadSkillFile("/nonexistent/file.md");
    expect(result).toBeNull();
  });
});

// ---- curator tests ----
describe("curator", () => {
  it("should record and retrieve executions", async () => {
    const { Curator } = await import("../src/curator");
    const curator = new Curator();
    curator.recordExecution({
      workflowId: "test-1",
      taskText: "build a login form",
      steps: ["planned", "implemented"],
      success: true,
      durationMs: 500,
      timestamp: new Date().toISOString(),
    });
    expect(curator.getExecutionHistory().length).toBe(1);
  });

  it("should generate a skill from repeated successful executions", async () => {
    const { Curator } = await import("../src/curator");
    const { SkillManager } = await import("../src/skill-system");
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "curator-test-"));
    const sm = new SkillManager(tmpDir);
    const curator = new Curator(sm);

    for (let i = 0; i < 3; i++) {
      curator.recordExecution({
        workflowId: `gen-${i}`,
        taskText: "optimize database query performance",
        steps: ["analyze", "index", "test"],
        success: true,
        durationMs: 100,
        timestamp: new Date().toISOString(),
      });
    }

    const report = await curator.runCycle();
    expect(report.newSkills).toBeGreaterThanOrEqual(1);
    fs.rmSync(tmpDir, { recursive: true });
  });
});

// ---- context-compressor tests ----
describe("context-compressor", () => {
  it("should apply tool budget to large outputs", async () => {
    const { applyToolBudget } = await import("../src/context-compressor");
    const messages = [
      { role: "tool", content: "x".repeat(10000) },
    ];
    const result = applyToolBudget(messages);
    expect(result.externalized.length).toBe(1);
    expect(result.messages[0].content.length).toBeLessThan(10000);
  });

  it("should skip small messages", async () => {
    const { applyToolBudget } = await import("../src/context-compressor");
    const messages = [
      { role: "user", content: "hi" },
      { role: "assistant", content: "hello" },
    ];
    const result = applyToolBudget(messages);
    expect(result.externalized.length).toBe(0);
  });

  it("should microcompact when there are many messages", async () => {
    const { microCompact } = await import("../src/context-compressor");
    const messages = [
      { role: "system", content: "You are an agent" },
      { role: "user", content: "do step 1" },
      { role: "assistant", content: "done step 1" },
      { role: "user", content: "do step 2" },
      { role: "assistant", content: "done step 2" },
      { role: "user", content: "do step 3" },
      { role: "assistant", content: "done step 3" },
    ];
    const result = microCompact(messages);
    expect(result.summary.length).toBeGreaterThan(0);
    expect(result.messages.length).toBeLessThan(messages.length);
  });
});

// ---- checkpoints tests ----
describe("checkpoints", () => {
  const testId = "ck-test-session";

  afterAll(async () => {
    const { deleteSession } = await import("../src/checkpoints");
    try { deleteSession(testId); } catch {}
  });

  it("should save and retrieve a checkpoint", async () => {
    const { saveCheckpoint, getLatestCheckpoint } = await import("../src/checkpoints");
    const ck = saveCheckpoint(testId, [
      { role: "system", content: "test" },
      { role: "user", content: "hello" },
    ], { iteration: 1 });
    expect(ck.sessionId).toBe(testId);
    expect(ck.messageCount).toBe(2);

    const retrieved = getLatestCheckpoint(testId);
    expect(retrieved).not.toBeNull();
    expect(retrieved!.sessionId).toBe(testId);
  });

  it("should list checkpoints with stats", async () => {
    const { listCheckpoints, getStorageStats } = await import("../src/checkpoints");
    const all = listCheckpoints();
    const stats = getStorageStats();
    expect(stats.totalCheckpoints).toBeGreaterThanOrEqual(1);
    expect(stats.sessions).toBeGreaterThanOrEqual(1);
  });

  it("should detect resumable sessions", async () => {
    const { hasResumableSession, getLastSessionId } = await import("../src/checkpoints");
    expect(hasResumableSession()).toBe(true);
    expect(getLastSessionId()).toBeTruthy();
  });
});

// ---- middleware-chain tests ----
describe("middleware-chain", () => {
  it("should contain 6 registered middlewares", async () => {
    const { globalMiddleware } = await import("../src/middleware-chain");
    const list = globalMiddleware.list();
    expect(list.length).toBe(6);
    const names = list.map((m) => m.name);
    expect(names).toContain("sandbox");
    expect(names).toContain("summarization");
    expect(names).toContain("memory");
    expect(names).toContain("todo");
  });

  it("should toggle middleware enable/disable", async () => {
    const { globalMiddleware } = await import("../src/middleware-chain");
    globalMiddleware.enable("title", true);
    const mw = globalMiddleware.get("title");
    expect(mw?.enabled).toBe(true);
    globalMiddleware.enable("title", false);
    expect(globalMiddleware.get("title")?.enabled).toBe(false);
  });

  it("should run before pipeline without errors", async () => {
    const { runBeforePipeline } = await import("../src/middleware-chain");
    const result = await runBeforePipeline({
      text: "hello",
      messages: [],
      tools: [],
      meta: {},
    });
    expect(result.earlyResponse).toBeNull();
    expect(result.modifiedCtx.text).toBe("hello");
  });
});

// ---- validate tests ----
describe("validate", () => {
  it("should accept valid agent execute body", async () => {
    const { validateAgentExecuteBody } = await import("../src/validate");
    expect(validateAgentExecuteBody({ text: "hello" })).toBeNull();
    expect(validateAgentExecuteBody({ text: "" })).toBe("text cannot be empty");
    expect(validateAgentExecuteBody({})).toBe("text must be string");
    expect(validateAgentExecuteBody(null)).toBe("body required");
  });

  it("should validate chat completions body", async () => {
    const { validateChatCompletionsBody } = await import("../src/validate");
    expect(validateChatCompletionsBody({ messages: [{ role: "user", content: "hi" }] })).toBeNull();
    expect(validateChatCompletionsBody({ messages: "not-array" })).toBe("messages must be an array");
  });
});
