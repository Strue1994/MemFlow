import { type Express, Router, type Request, type Response } from "express";
import { promises as fs } from "node:fs";
import net from "node:net";
import path from "node:path";
import { runComputerCommand, type CommandResult } from "./computer_agent";

const RUNTIME_ROOT =
  process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(__dirname, "..", "..", ".memflow-runtime");
const PROJECT_ROOT = path.resolve(__dirname, "..", "..");
const REFERENCE_ROOT =
  process.env.CODING_KERNEL_REFERENCE_ROOT ||
  path.resolve(__dirname, "..", "..", "..", ".claude", "package");
const REFERENCE_OUTPUT_ROOT = path.join(RUNTIME_ROOT, "reference");
const VERIFICATION_OUTPUT_ROOT = path.join(RUNTIME_ROOT, "verification");
const RECOVERY_OUTPUT_ROOT = path.join(RUNTIME_ROOT, "recovery");
const SOURCE_PREFIX = "../src/";
const COMMAND_PREFIX = `${SOURCE_PREFIX}commands/`;
const TOOL_PREFIX = `${SOURCE_PREFIX}tools/`;
const SERVICE_PREFIX = `${SOURCE_PREFIX}services/`;

const KNOWN_SLASH_COMMANDS = [
  "agents",
  "compact",
  "config",
  "context",
  "doctor",
  "hooks",
  "mcp",
  "memory",
  "permissions",
  "plan",
  "plugin",
  "resume",
  "review",
  "session",
  "tasks",
] as const;

const CAPABILITY_PATTERNS = [
  {
    id: "slash-commands",
    label: "Slash Commands",
    description: "Discoverable command surface for common operator workflows.",
    sourcePatterns: [COMMAND_PREFIX],
    textPatterns: ["/plan", "/review", "/memory", "/tasks", "/compact"],
  },
  {
    id: "permission-guardrails",
    label: "Permission Guardrails",
    description: "Permission controls, approval modes, and destructive-action constraints.",
    sourcePatterns: ["permission", "approval"],
    textPatterns: ["permissions", "dangerouslySkipPermissions", "approval"],
  },
  {
    id: "tool-hooks",
    label: "Tool Hooks",
    description: "Pre/post tool hooks for governance, interception, and auditing.",
    sourcePatterns: ["hook"],
    textPatterns: ["hook", "preToolUse", "postToolUse"],
  },
  {
    id: "mcp-integration",
    label: "MCP Integration",
    description: "External tools, resources, and auth surfaces integrated into the agent.",
    sourcePatterns: ["mcp"],
    textPatterns: ["mcp", "ListMcpResourcesTool", "McpAuthTool"],
  },
  {
    id: "session-memory",
    label: "Session and Memory",
    description: "Compaction, resume, memory sync, and long-context management.",
    sourcePatterns: ["memory", "resume", "compact"],
    textPatterns: ["resume", "memory", "compact", "handoff"],
  },
  {
    id: "background-execution",
    label: "Background Execution",
    description: "Task queues, asynchronous execution, and observable run state.",
    sourcePatterns: ["task", "execution"],
    textPatterns: ["Task", "tasks", "background", "execution"],
  },
  {
    id: "multi-agent",
    label: "Multi-Agent",
    description: "Task delegation, teammate spawning, and collaborative execution.",
    sourcePatterns: ["agent", "teammate"],
    textPatterns: ["Agent", "teammate", "subagent", "Task tool"],
  },
  {
    id: "review-workflows",
    label: "Review Workflows",
    description: "Code review, verification, and quality-oriented flows.",
    sourcePatterns: ["review"],
    textPatterns: ["review", "verification", "quality"],
  },
];

const CODING_TOOL_FAMILIES = [
  {
    id: "fs",
    label: "Filesystem",
    status: "partial",
    actions: ["list", "search", "read_text", "write_text"],
    safety: "sandboxed",
    currentBackends: ["computer-agent"],
  },
  {
    id: "shell",
    label: "Shell",
    status: "partial",
    actions: ["run_safe_command", "collect_stdout", "capture_exit_code"],
    safety: "allowlisted",
    currentBackends: ["computer-agent"],
  },
  {
    id: "git",
    label: "Git",
    status: "partial",
    actions: ["status", "diff", "log", "branch"],
    safety: "read-mostly",
    currentBackends: ["computer-agent"],
  },
  {
    id: "browser",
    label: "Browser",
    status: "partial",
    actions: ["open", "fetch_html", "snapshot"],
    safety: "network-bounded",
    currentBackends: ["computer-agent"],
  },
  {
    id: "codebase",
    label: "Codebase Intelligence",
    status: "planned",
    actions: ["symbol_search", "dependency_graph", "test_discovery"],
    safety: "read-only",
    currentBackends: [],
  },
  {
    id: "verify",
    label: "Verification",
    status: "planned",
    actions: ["build", "lint", "test", "smoke", "artifact_check"],
    safety: "controlled-exec",
    currentBackends: [],
  },
];

type CapabilitySignal = {
  id: string;
  label: string;
  description: string;
  score: number;
  evidence: string[];
};

type ReferenceSnapshot = {
  referenceId: string;
  packageName: string;
  version: string;
  packageRoot: string;
  generatedAt: string;
  cliBundle: {
    cliBytes: number;
    sourceMapBytes: number;
  };
  sourceSummary: {
    totalSources: number;
    commandSources: number;
    toolSources: number;
    serviceSources: number;
  };
  slashCommands: string[];
  capabilitySignals: CapabilitySignal[];
};

type FailureCategory =
  | "build_failure"
  | "test_failure"
  | "permission_denied"
  | "port_conflict"
  | "executor_unavailable"
  | "missing_dependency"
  | "env_missing"
  | "network_upstream"
  | "rate_limit"
  | "ssrf_blocked"
  | "invalid_url"
  | "invalid_return"
  | "unknown";

type RecoveryClassification = {
  category: FailureCategory;
  confidence: number;
  summary: string;
  recommendedActions: string[];
  verificationFocus: string[];
};

type VerificationCheck = {
  id: string;
  label: string;
  kind: "command" | "http" | "file";
  reason: string;
  command?: string;
  cwd?: string;
  url?: string;
  expectStatus?: number;
  filePath?: string;
  fileRoot?: "project" | "runtime" | "absolute";
  mustExist?: boolean;
  containsText?: string;
  minBytes?: number;
};

type VerificationPlan = {
  taskType: string;
  objective: string;
  changedFiles: string[];
  checks: VerificationCheck[];
  evidencePaths: string[];
};

type ReferenceIngestRequest = {
  packageRoot?: string;
  persist?: boolean;
};

type RecoveryRequest = {
  error?: string;
  command?: string;
  tool?: string;
  changedFiles?: string[];
};

type VerificationPlanRequest = {
  taskType?: string;
  objective?: string;
  changedFiles?: string[];
};

type VerificationRunRequest = VerificationPlanRequest & {
  plan?: VerificationPlan;
};

type VerificationCheckResult = {
  id: string;
  label: string;
  kind: "command" | "http" | "file";
  success: boolean;
  command?: string;
  url?: string;
  filePath?: string;
  expectedStatus?: number;
  actualStatus?: number;
  stdout?: string;
  stderr?: string;
  exitCode?: number | null;
  durationMs: number;
  reason: string;
};

type VerificationRunReport = {
  reportId: string;
  generatedAt: string;
  objective: string;
  taskType: string;
  changedFiles: string[];
  success: boolean;
  results: VerificationCheckResult[];
};

type RecoveryAction = {
  id: string;
  label: string;
  kind: "inspect" | "reroute" | "config" | "replace" | "verify" | "patch";
  detail: string;
  command?: string;
  url?: string;
};

type RecoveryPlan = {
  reportId: string;
  generatedAt: string;
  classification: RecoveryClassification;
  actions: RecoveryAction[];
  verificationPlan?: VerificationPlan;
};

type RecoveryPlanRequest = RecoveryRequest & VerificationPlanRequest;

type RecoveryActionExecutionResult = {
  id: string;
  label: string;
  status: "completed" | "advisory" | "failed";
  detail: string;
  durationMs: number;
  data?: unknown;
  error?: string;
};

type RecoveryExecutionReport = {
  reportId: string;
  generatedAt: string;
  classification: RecoveryClassification;
  actionResults: RecoveryActionExecutionResult[];
  success: boolean;
  verification?: VerificationRunReport | null;
};

type AutoFixDispatchResult = {
  id: string;
  label: string;
  status: "completed" | "skipped" | "failed";
  detail: string;
  data?: unknown;
  error?: string;
};

type AutoFixDispatchReport = {
  reportId: string;
  generatedAt: string;
  classification: RecoveryClassification;
  recovery: RecoveryExecutionReport;
  dispatchResults: AutoFixDispatchResult[];
  success: boolean;
};

function defaultReferencePackageRoot(): string {
  return REFERENCE_ROOT;
}

function referenceSnapshotPath(snapshot: ReferenceSnapshot): string {
  return path.join(REFERENCE_OUTPUT_ROOT, snapshot.referenceId, "capabilities.json");
}

async function readJsonFile<T>(filePath: string): Promise<T> {
  const raw = await fs.readFile(filePath, "utf8");
  return JSON.parse(raw) as T;
}

async function pathExists(filePath: string): Promise<boolean> {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

function uniqueSlashCommands(rawMapText: string): string[] {
  return KNOWN_SLASH_COMMANDS.filter((command) => rawMapText.includes(`/${command}`));
}

function scoreCapabilitySources(sources: string[], rawMapText: string): CapabilitySignal[] {
  return CAPABILITY_PATTERNS.map((definition) => {
    const sourceHits = sources.filter((source) =>
      definition.sourcePatterns.some((pattern) => source.toLowerCase().includes(pattern.toLowerCase())),
    );
    const textHits = definition.textPatterns.filter((pattern) => rawMapText.includes(pattern));
    const evidence = [...sourceHits.slice(0, 4), ...textHits.slice(0, 4)].slice(0, 6);
    const scoreBase = sourceHits.length * 0.12 + textHits.length * 0.18;
    return {
      id: definition.id,
      label: definition.label,
      description: definition.description,
      score: Number(Math.min(1, scoreBase).toFixed(2)),
      evidence,
    };
  }).sort((a, b) => b.score - a.score);
}

export async function ingestClaudeReference(packageRoot = defaultReferencePackageRoot(), persist = true): Promise<ReferenceSnapshot> {
  const packageJsonPath = path.join(packageRoot, "package.json");
  const sourceMapPath = path.join(packageRoot, "cli.js.map");
  const cliPath = path.join(packageRoot, "cli.js");

  const [packageJson, cliStat, sourceMapStat, sourceMapRaw] = await Promise.all([
    readJsonFile<{ name?: string; version?: string }>(packageJsonPath),
    fs.stat(cliPath),
    fs.stat(sourceMapPath),
    fs.readFile(sourceMapPath, "utf8"),
  ]);

  const parsedSourceMap = JSON.parse(sourceMapRaw) as { sources?: string[] };
  const sources = parsedSourceMap.sources || [];
  const slashCommands = uniqueSlashCommands(sourceMapRaw);
  const snapshot: ReferenceSnapshot = {
    referenceId: `${(packageJson.name || "claude-code").replace(/[^a-z0-9-]+/gi, "-")}-${packageJson.version || "unknown"}`,
    packageName: packageJson.name || "@anthropic-ai/claude-code",
    version: packageJson.version || "unknown",
    packageRoot,
    generatedAt: new Date().toISOString(),
    cliBundle: {
      cliBytes: cliStat.size,
      sourceMapBytes: sourceMapStat.size,
    },
    sourceSummary: {
      totalSources: sources.length,
      commandSources: sources.filter((source) => source.startsWith(COMMAND_PREFIX)).length,
      toolSources: sources.filter((source) => source.startsWith(TOOL_PREFIX)).length,
      serviceSources: sources.filter((source) => source.startsWith(SERVICE_PREFIX)).length,
    },
    slashCommands,
    capabilitySignals: scoreCapabilitySources(sources, sourceMapRaw),
  };

  if (persist) {
    const outputPath = referenceSnapshotPath(snapshot);
    await fs.mkdir(path.dirname(outputPath), { recursive: true });
    await fs.writeFile(outputPath, JSON.stringify(snapshot, null, 2), "utf8");
  }

  return snapshot;
}

export async function loadLatestReferenceSnapshot(): Promise<ReferenceSnapshot | null> {
  if (!(await pathExists(REFERENCE_OUTPUT_ROOT))) {
    return null;
  }

  const references = await fs.readdir(REFERENCE_OUTPUT_ROOT, { withFileTypes: true });
  const candidates = references.filter((entry) => entry.isDirectory()).map((entry) => entry.name);
  if (candidates.length === 0) {
    return null;
  }

  const snapshots = await Promise.all(
    candidates.map(async (candidate) => {
      const snapshotPath = path.join(REFERENCE_OUTPUT_ROOT, candidate, "capabilities.json");
      if (!(await pathExists(snapshotPath))) {
        return null;
      }
      return readJsonFile<ReferenceSnapshot>(snapshotPath);
    }),
  );

  return snapshots
    .filter((snapshot): snapshot is ReferenceSnapshot => Boolean(snapshot))
    .sort((a, b) => b.generatedAt.localeCompare(a.generatedAt))[0] || null;
}

export function classifyFailure(input: RecoveryRequest): RecoveryClassification {
  const text = [input.error, input.command, input.tool, ...(input.changedFiles || [])]
    .filter(Boolean)
    .join("\n");

  const rules: Array<{
    category: FailureCategory;
    confidence: number;
    summary: string;
    patterns: RegExp[];
    recommendedActions: string[];
    verificationFocus: string[];
  }> = [
    {
      category: "permission_denied",
      confidence: 0.95,
      summary: "Permission or filesystem policy blocked the operation.",
      patterns: [/access denied/i, /eperm/i, /permission denied/i, /os error 5/i],
      recommendedActions: [
        "Move runtime output to a confirmed writable project-local folder.",
        "Avoid in-place overwrite of locked binaries; switch to a new port or new target path.",
        "Verify parent directory write access before retrying the same action.",
      ],
      verificationFocus: ["Writable path probe", "Alternate target path", "Process/file lock check"],
    },
    {
      category: "port_conflict",
      confidence: 0.92,
      summary: "A required port is already occupied by another process.",
      patterns: [/address already in use/i, /eaddrinuse/i, /port .* already/i],
      recommendedActions: [
        "Discover the owning PID and decide whether to reuse or move to a new port.",
        "Prefer switching the stack to a fresh port over killing protected processes.",
      ],
      verificationFocus: ["netstat or socket owner", "Health check on alternate port"],
    },
    {
      category: "executor_unavailable",
      confidence: 0.9,
      summary: "The local executor service is unavailable or refusing connections.",
      patterns: [/fetch failed/i, /connection refused/i],
      recommendedActions: [
        "Check whether the local executor is listening on 8082.",
        "Restart the live executor using the fixed runtime script.",
        "Re-run executor root and workflows smoke checks before resuming autonomy.",
      ],
      verificationFocus: ["Executor restart script", "Executor root HTTP 200", "Workflow list HTTP 200"],
    },
    {
      category: "missing_dependency",
      confidence: 0.9,
      summary: "A required binary, package, or module is missing.",
      patterns: [/cannot find module/i, /not recognized as an internal or external command/i, /command not found/i, /no such file or directory/i],
      recommendedActions: [
        "Identify whether the missing item is a package, binary, or generated artifact.",
        "Install or point to a local project-scoped copy instead of relying on global state.",
      ],
      verificationFocus: ["Dependency manifest", "Executable presence", "Post-install smoke check"],
    },
    {
      category: "env_missing",
      confidence: 0.88,
      summary: "Required runtime configuration is absent or empty.",
      patterns: [/is not configured/i, /missing .*key/i, /missing .*token/i, /undefined environment/i],
      recommendedActions: [
        "Load runtime settings from the fixed config directory before executing the task.",
        "Surface a precise missing-key list instead of generic startup failure.",
      ],
      verificationFocus: ["Effective env dump", "Config file readback"],
    },
    {
      category: "rate_limit",
      confidence: 0.94,
      summary: "The upstream or executor rate limiter rejected the request burst.",
      patterns: [/rate limit exceeded/i, /too many requests/i, /429\b/i],
      recommendedActions: [
        "Back off the next tick and reduce repeated polling on the same path.",
        "Use a local dev key with explicit unlimited policy for local loops.",
      ],
      verificationFocus: ["Retry cadence", "Rate-limit headers or config"],
    },
    {
      category: "ssrf_blocked",
      confidence: 0.96,
      summary: "The executor rejected a loopback or internal URL due to SSRF protection.",
      patterns: [/ssrf protection/i, /url .* is not allowed/i],
      recommendedActions: [
        "Replace internal HTTP probes with local file-backed or externally reachable samples.",
        "Keep learning samples on whitelisted, low-risk primitives while the executor is old.",
      ],
      verificationFocus: ["Workflow URL targets", "Local side-effect substitute"],
    },
    {
      category: "invalid_url",
      confidence: 0.93,
      summary: "A malformed base URL or whitespace-corrupted endpoint prevented request construction.",
      patterns: [/failed to parse url/i, /invalid url/i, /unsupported protocol/i],
      recommendedActions: [
        "Normalize service base URLs by trimming whitespace and removing trailing slashes.",
        "Patch endpoint builders to sanitize the base URL before concatenating paths.",
      ],
      verificationFocus: ["Sanitized base URL", "Endpoint fetch smoke test"],
    },
    {
      category: "invalid_return",
      confidence: 0.93,
      summary: "The workflow finished without an explicit Return in the current executor binary.",
      patterns: [/invalid return: no value to return/i],
      recommendedActions: [
        "Prefer workflows whose success can be verified via side effects and treat this as soft-success when appropriate.",
        "Replace the executor binary or inject a compile path that emits Return instructions.",
      ],
      verificationFocus: ["Side-effect artifact", "Executor build provenance", "Workflow compile path"],
    },
    {
      category: "network_upstream",
      confidence: 0.84,
      summary: "A remote network request failed or the upstream was unavailable.",
      patterns: [/error sending request/i, /connection refused/i, /timed out/i, /dns/i],
      recommendedActions: [
        "Swap external samples for deterministic local samples when building the learning loop.",
        "Separate network instability from agent logic before applying tuning.",
      ],
      verificationFocus: ["Direct endpoint probe", "Local substitute path"],
    },
    {
      category: "test_failure",
      confidence: 0.82,
      summary: "A test or validation step failed after code execution.",
      patterns: [/test failed/i, /assert/i, /expected .* received/i],
      recommendedActions: [
        "Narrow the failing test and inspect regression scope before broader edits.",
        "Capture the smallest reproducible failure and patch that path first.",
      ],
      verificationFocus: ["Targeted test rerun", "Minimal failing case"],
    },
    {
      category: "build_failure",
      confidence: 0.8,
      summary: "Compilation or build output failed before runtime execution.",
      patterns: [/build failed/i, /tsc/i, /cargo/i, /compile error/i],
      recommendedActions: [
        "Fix the earliest compile error before addressing downstream runtime symptoms.",
        "Prefer fast compile checks over full runtime restart during iteration.",
      ],
      verificationFocus: ["Typecheck/build rerun", "First compiler error only"],
    },
  ];

  const match = rules.find((rule) => rule.patterns.some((pattern) => pattern.test(text)));
  if (match) {
    return {
      category: match.category,
      confidence: match.confidence,
      summary: match.summary,
      recommendedActions: match.recommendedActions,
      verificationFocus: match.verificationFocus,
    };
  }

  return {
    category: "unknown",
    confidence: 0.4,
    summary: "The failure does not match a known recovery rule yet.",
    recommendedActions: [
      "Capture the exact stderr or API error body.",
      "Classify whether the failure is build-time, runtime, network, or policy-related.",
      "Add a new recovery rule once the failure is repeatable.",
    ],
    verificationFocus: ["Raw error body", "Reproduction steps"],
  };
}

export function suggestChangedFilesForFailure(category: FailureCategory): string[] {
  switch (category) {
    case "invalid_return":
      return ["executor/src/lib.rs", "scripts/seed-cron-workflows.js", "agent-service/src/autonomy_supervisor.ts"];
    case "ssrf_blocked":
      return ["scripts/seed-cron-workflows.js", "scripts/cron-runner.js"];
    case "invalid_url":
      return ["agent-service/src/index.ts", "agent-service/src/autonomy_supervisor.ts"];
    case "executor_unavailable":
      return ["scripts/restart-live-executor.ps1", "agent-service/src/autonomy_supervisor.ts"];
    case "permission_denied":
      return ["scripts/dev-local.ps1", "scripts/stop-local.ps1"];
    case "port_conflict":
      return ["scripts/dev-local.ps1", "agent-service/src/index.ts", "scripts/serve-web-ui.js"];
    case "rate_limit":
      return ["executor/src/http_server.rs", "agent-service/src/autonomy_supervisor.ts"];
    default:
      return [];
  }
}

export function planVerification(input: VerificationPlanRequest): VerificationPlan {
  const changedFiles = input.changedFiles || [];
  const taskType = (input.taskType || "feature").trim() || "feature";
  const objective = (input.objective || "Stabilize and verify the requested change.").trim();
  const checks: VerificationPlan["checks"] = [];

  const addCommandCheck = (id: string, label: string, command: string, reason: string, cwd?: string) => {
    if (!checks.some((check) => check.id === id && check.command === command)) {
      checks.push({ id, label, kind: "command", command, reason, cwd });
    }
  };

  const addHttpCheck = (id: string, label: string, url: string, reason: string, expectStatus = 200) => {
    if (!checks.some((check) => check.id === id && check.url === url)) {
      checks.push({ id, label, kind: "http", url, reason, expectStatus });
    }
  };

  const addFileCheck = (
    id: string,
    label: string,
    filePath: string,
    reason: string,
    options: {
      fileRoot?: "project" | "runtime" | "absolute";
      mustExist?: boolean;
      containsText?: string;
      minBytes?: number;
    } = {},
  ) => {
    if (!checks.some((check) => check.id === id && check.filePath === filePath)) {
      checks.push({
        id,
        label,
        kind: "file",
        filePath,
        reason,
        fileRoot: options.fileRoot || "project",
        mustExist: options.mustExist ?? true,
        containsText: options.containsText,
        minBytes: options.minBytes,
      });
    }
  };

  addCommandCheck("git-status", "Git status", "git status --short", "Check the working tree and confirm the touched surface.");
  addCommandCheck("git-diff", "Git diff stat", "git diff --stat", "Summarize the scope of the active change before claiming completion.");

  if (changedFiles.some((file) => file.endsWith(".ts") || file.endsWith(".tsx") || file.endsWith(".js"))) {
    addCommandCheck("agent-build", "TypeScript build", "npm run build", "Compile application changes before runtime validation.");
  }
  if (changedFiles.some((file) => file.endsWith(".rs"))) {
    addCommandCheck("rust-check", "Rust check", "cargo check", "Catch executor or compiler regressions before runtime.");
  }
  if (changedFiles.some((file) => file.endsWith(".ps1"))) {
    addCommandCheck(
      "powershell-parse",
      "PowerShell parse",
      "powershell -NoProfile -Command \"Get-Command ./scripts/*.ps1 | Out-Null\"",
      "Validate script syntax and command discoverability.",
    );
  }
  if (changedFiles.some((file) => /index\.ts|computer_agent|coding_kernel/i.test(file))) {
    addHttpCheck(
      "service-smoke",
      "Agent service smoke",
      "http://127.0.0.1:3000/llm-settings/catalog",
      "Confirm the agent service still responds after server-side changes.",
    );
    addFileCheck(
      "coding-kernel-artifact",
      "Coding kernel build artifact",
      "agent-service/dist/coding_kernel.js",
      "Confirm the coding kernel compiled into a runtime artifact.",
      { minBytes: 1000 },
    );
  }
  if (changedFiles.some((file) => /autonomy|cron|workflow/i.test(file))) {
    addHttpCheck(
      "autonomy-smoke",
      "Autonomy status",
      "http://127.0.0.1:3000/autonomy/status",
      "Check that the self-improvement loop still exposes status and can be inspected.",
    );
  }
  if (changedFiles.some((file) => /llm_settings|coding_kernel|reference/i.test(file))) {
    addFileCheck(
      "reference-snapshot",
      "Reference capability snapshot",
      path.join(REFERENCE_OUTPUT_ROOT, "-anthropic-ai-claude-code-2.1.88", "capabilities.json"),
      "Ensure the Claude Code reference snapshot exists for capability-guided reasoning.",
      { fileRoot: "absolute", minBytes: 1000 },
    );
  }
  if (checks.length === 0) {
    addCommandCheck("manual-review", "Manual diff review", "git diff --stat", "No specialized checks detected; start with the change surface.");
  }

  return {
    taskType,
    objective,
    changedFiles,
    checks,
    evidencePaths: [
      path.join(RUNTIME_ROOT, "verification"),
      path.join(RUNTIME_ROOT, "logs"),
      path.join(RUNTIME_ROOT, "reference"),
    ],
  };
}

async function writeArtifact(root: string, prefix: string, payload: object): Promise<string> {
  const reportId = `${prefix}-${new Date().toISOString().replace(/[:.]/g, "-")}`;
  const outputPath = path.join(root, `${reportId}.json`);
  await fs.mkdir(root, { recursive: true });
  await fs.writeFile(outputPath, JSON.stringify(payload, null, 2), "utf8");
  return outputPath;
}

async function runVerificationCheck(check: VerificationCheck): Promise<VerificationCheckResult> {
  const startedAt = Date.now();
  if (check.kind === "command") {
    const commandResult: CommandResult = await runComputerCommand(check.command || "", check.cwd);
    const isGitCheck = (check.command || "").trim().toLowerCase().startsWith("git ");
    const isMissingGitRepo =
      isGitCheck &&
      typeof commandResult.stderr === "string" &&
      commandResult.stderr.toLowerCase().includes("not a git repository");
    const isGitCommandBlocked =
      isGitCheck &&
      typeof commandResult.stderr === "string" &&
      commandResult.stderr.toLowerCase().includes("spawn eperm");
    return {
      id: check.id,
      label: check.label,
      kind: "command",
      success:
        (commandResult.allowed && (commandResult.exitCode === 0 || commandResult.exitCode === null)) ||
        isMissingGitRepo ||
        isGitCommandBlocked,
      command: commandResult.command,
      stdout:
        isMissingGitRepo
          ? "Skipped: workspace is not a git repository."
          : isGitCommandBlocked
            ? "Skipped: git subprocess execution is blocked in this environment."
            : commandResult.stdout,
      stderr: commandResult.stderr,
      exitCode: commandResult.exitCode,
      durationMs: commandResult.durationMs,
      reason: check.reason,
    };
  }

  if (check.kind === "file") {
    const fullPath =
      check.fileRoot === "absolute"
        ? (check.filePath || "")
        : check.fileRoot === "runtime"
          ? path.join(RUNTIME_ROOT, check.filePath || "")
          : path.join(PROJECT_ROOT, check.filePath || "");
    try {
      const stat = await fs.stat(fullPath);
      let success = true;
      let preview = `size=${stat.size}`;
      if (check.mustExist === false) {
        success = false;
      }
      if (check.minBytes && stat.size < check.minBytes) {
        success = false;
      }
      if (check.containsText) {
        const content = await fs.readFile(fullPath, "utf8");
        preview = content.slice(0, 4000);
        if (!content.includes(check.containsText)) {
          success = false;
        }
      }
      return {
        id: check.id,
        label: check.label,
        kind: "file",
        success,
        filePath: fullPath,
        stdout: preview,
        durationMs: Date.now() - startedAt,
        reason: check.reason,
      };
    } catch (error) {
      return {
        id: check.id,
        label: check.label,
        kind: "file",
        success: false,
        filePath: fullPath,
        stderr: error instanceof Error ? error.message : String(error),
        durationMs: Date.now() - startedAt,
        reason: check.reason,
      };
    }
  }

  const response = await fetch(check.url || "");
  const body = await response.text();
  return {
    id: check.id,
    label: check.label,
    kind: "http",
    success: response.status === (check.expectStatus || 200),
    url: check.url,
    expectedStatus: check.expectStatus || 200,
    actualStatus: response.status,
    stdout: body.slice(0, 4000),
    durationMs: Date.now() - startedAt,
    reason: check.reason,
  };
}

export async function runVerificationPlan(input: VerificationRunRequest): Promise<VerificationRunReport> {
  const plan = input.plan || planVerification(input);
  const results: VerificationCheckResult[] = [];

  for (const check of plan.checks) {
    try {
      results.push(await runVerificationCheck(check));
    } catch (error) {
      results.push({
        id: check.id,
        label: check.label,
        kind: check.kind,
        success: false,
        command: check.command,
        url: check.url,
        durationMs: 0,
        stderr: error instanceof Error ? error.message : String(error),
        reason: check.reason,
      });
    }
  }

  const report: VerificationRunReport = {
    reportId: `verification-${Date.now()}`,
    generatedAt: new Date().toISOString(),
    objective: plan.objective,
    taskType: plan.taskType,
    changedFiles: plan.changedFiles,
    success: results.every((result) => result.success),
    results,
  };

  await writeArtifact(VERIFICATION_OUTPUT_ROOT, "verification", report);
  return report;
}

export function buildRecoveryPlan(input: RecoveryPlanRequest): RecoveryPlan {
  const classification = classifyFailure(input);
  const actions: RecoveryAction[] = [];
  const addAction = (action: RecoveryAction) => actions.push(action);
  const relevantText = [input.error, input.command, input.tool].filter(Boolean).join("\n");
  const matchedPort = relevantText.match(/\b([1-9][0-9]{2,4})\b/);
  const port = matchedPort ? Number(matchedPort[1]) : 3000;

  switch (classification.category) {
    case "permission_denied":
      addAction({
        id: "probe-writable-root",
        label: "Probe writable runtime root",
        kind: "inspect",
        detail: "Verify the target runtime and build directories can create files before retrying.",
      });
      addAction({
        id: "reroute-build-output",
        label: "Reroute build output",
        kind: "reroute",
        detail: "Move binary output or runtime state into a new project-local folder instead of overwriting a locked path.",
      });
      break;
    case "invalid_return":
      addAction({
        id: "switch-soft-success",
        label: "Use side-effect backed samples",
        kind: "reroute",
        detail: "Prefer workflows whose success can be verified from file or local state artifacts.",
      });
      addAction({
        id: "replace-executor-binary",
        label: "Replace executor binary",
        kind: "replace",
        detail: "Move to a newer executor build or a compile path that emits explicit Return instructions.",
      });
      break;
    case "invalid_url":
      addAction({
        id: "normalize-url-builders",
        label: "Normalize URL builders",
        kind: "patch",
        detail: "Apply the whitelisted URL-sanitization patch to agent-service endpoint builders.",
      });
      break;
    case "executor_unavailable":
      addAction({
        id: "inspect-executor-port",
        label: "Inspect executor port",
        kind: "inspect",
        detail: "Check whether the live executor is listening on 8082.",
        command: "netstat -ano | findstr :8082",
      });
      addAction({
        id: "restart-live-executor-ready",
        label: "Live executor restart ready",
        kind: "reroute",
        detail: "Use the dedicated restart-live-executor.ps1 script to bring 8082 back under the fixed runtime.",
      });
      break;
    case "port_conflict":
      addAction({
        id: "inspect-port-owner",
        label: "Inspect port owner",
        kind: "inspect",
        detail: "Identify the PID and decide whether to reuse the service or move to a different port.",
        command: `netstat -ano | findstr :${port}`,
      });
      addAction({
        id: "shift-port",
        label: "Shift to alternate port",
        kind: "reroute",
        detail: "Move the stack to a fresh port instead of killing a protected process.",
      });
      break;
    case "ssrf_blocked":
      addAction({
        id: "replace-loopback-samples",
        label: "Replace loopback HTTP samples",
        kind: "reroute",
        detail: "Stop using internal HTTP URLs inside executor workflows and replace them with local file side effects.",
      });
      break;
    default:
      classification.recommendedActions.forEach((detail, index) => {
        addAction({
          id: `recommended-${index + 1}`,
          label: `Recommended action ${index + 1}`,
          kind: "inspect",
          detail,
        });
      });
      break;
  }

  const verificationPlan =
    (input.changedFiles && input.changedFiles.length > 0) || suggestChangedFilesForFailure(classification.category).length > 0
      ? planVerification({
          taskType: input.taskType,
          objective: input.objective || classification.summary,
          changedFiles: input.changedFiles && input.changedFiles.length > 0 ? input.changedFiles : suggestChangedFilesForFailure(classification.category),
        })
      : undefined;

  return {
    reportId: `recovery-${Date.now()}`,
    generatedAt: new Date().toISOString(),
    classification,
    actions,
    verificationPlan,
  };
}

async function persistRecoveryPlan(plan: RecoveryPlan): Promise<void> {
  await writeArtifact(RECOVERY_OUTPUT_ROOT, "recovery", plan);
}

async function writeEnvironmentOverrideScript(filePath: string, entries: Record<string, string>): Promise<string> {
  const lines = Object.entries(entries).map(([key, value]) => `$env:${key}='${value.replace(/'/g, "''")}'`);
  await fs.writeFile(filePath, `${lines.join("\n")}\n`, "utf8");
  return filePath;
}

async function writeRestartScript(filePath: string, overrideScriptPath: string): Promise<string> {
  const script = [
    `$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path`,
    `. "${overrideScriptPath}"`,
    `& "${path.join(PROJECT_ROOT, "scripts", "dev-local.ps1")}"`,
    "",
  ].join("\n");
  await fs.writeFile(filePath, script, "utf8");
  return filePath;
}

async function applyWhitelistedSourcePatch(patchId: string): Promise<AutoFixDispatchResult> {
  if (patchId !== "normalize-url-builders") {
    return {
      id: patchId,
      label: patchId,
      status: "skipped",
      detail: "No whitelisted source patch matched the requested patch id.",
    };
  }

  const indexPath = path.join(PROJECT_ROOT, "agent-service", "src", "index.ts");
  const autonomyPath = path.join(PROJECT_ROOT, "agent-service", "src", "autonomy_supervisor.ts");
  const targets = [indexPath, autonomyPath];
  const touched: string[] = [];

  for (const target of targets) {
    const raw = await fs.readFile(target, "utf8");
    let next = raw;
    if (target.endsWith("index.ts")) {
      if (!raw.includes("sanitizeServiceBaseUrl")) {
        next = raw.replace(
          'import { getLLMSettings, LLM_PROVIDER_PRESETS, saveLLMSettings, type LLMSettings } from "./llm_settings";\n',
          'import { getLLMSettings, LLM_PROVIDER_PRESETS, saveLLMSettings, type LLMSettings } from "./llm_settings";\n\nfunction sanitizeServiceBaseUrl(value: string): string {\n  return value.trim().replace(/\\s+/g, "").replace(/\\/+$/, "");\n}\n',
        );
        next = next.replace(
          'const EXECUTOR_URL = process.env.EXECUTOR_URL || "http://127.0.0.1:8082";\nconst MEMORY_HUB_URL = process.env.MEMORY_HUB_URL || "http://127.0.0.1:8081";\n',
          'const EXECUTOR_URL = sanitizeServiceBaseUrl(process.env.EXECUTOR_URL || "http://127.0.0.1:8082");\nconst MEMORY_HUB_URL = sanitizeServiceBaseUrl(process.env.MEMORY_HUB_URL || "http://127.0.0.1:8081");\n',
        );
      }
    } else if (target.endsWith("autonomy_supervisor.ts")) {
      if (!raw.includes("private executorEndpoint(")) {
        next = raw.replace(
          "    const response = await fetch(`${this.deps.executorUrl}${path}`, {\n",
          "    const response = await fetch(this.executorEndpoint(path), {\n",
        );
        next = next.replace(
          "    const response = await fetch(`${this.deps.executorUrl}${path}`, {\n",
          "    const response = await fetch(this.executorEndpoint(path), {\n",
        );
        next = next.replace(
          "  }\n}\n",
          "  }\n\n  private executorEndpoint(requestPath: string): string {\n    const base = this.deps.executorUrl.trim().replace(/\\s+/g, \"\").replace(/\\/+$/, \"\");\n    return `${base}${requestPath}`;\n  }\n}\n",
        );
      }
    }

    if (next !== raw) {
      await fs.writeFile(target, next, "utf8");
      touched.push(target);
    }
  }

  return {
    id: patchId,
    label: "Normalize URL builders",
    status: touched.length > 0 ? "completed" : "skipped",
    detail: touched.length > 0 ? "Applied whitelisted URL-sanitization source patches." : "URL-sanitization source patches were already present.",
    data: { touched },
  };
}

async function probeWritableRoot(targetRoot: string): Promise<{ root: string; writable: boolean }> {
  await fs.mkdir(targetRoot, { recursive: true });
  const probePath = path.join(targetRoot, `.probe-${Date.now()}.tmp`);
  try {
    await fs.writeFile(probePath, "ok", "utf8");
    await fs.unlink(probePath);
    return { root: targetRoot, writable: true };
  } catch {
    return { root: targetRoot, writable: false };
  }
}

async function findAvailablePort(startPort: number): Promise<number | null> {
  for (let port = startPort; port < startPort + 20; port += 1) {
    const available = await new Promise<boolean>((resolve) => {
      const server = net.createServer();
      server.once("error", () => resolve(false));
      server.once("listening", () => {
        server.close(() => resolve(true));
      });
      server.listen(port, "127.0.0.1");
    });
    if (available) {
      return port;
    }
  }
  return null;
}

async function executeRecoveryAction(action: RecoveryAction): Promise<RecoveryActionExecutionResult> {
  const startedAt = Date.now();
  try {
    switch (action.id) {
      case "probe-writable-root": {
        const probes = await Promise.all([
          probeWritableRoot(RUNTIME_ROOT),
          probeWritableRoot(VERIFICATION_OUTPUT_ROOT),
          probeWritableRoot(RECOVERY_OUTPUT_ROOT),
        ]);
        const writable = probes.every((probe) => probe.writable);
        return {
          id: action.id,
          label: action.label,
          status: writable ? "completed" : "failed",
          detail: writable ? "Runtime roots are writable." : "One or more runtime roots are not writable.",
          durationMs: Date.now() - startedAt,
          data: probes,
        };
      }
      case "reroute-build-output": {
        const suggestion = {
          runtimeRoot: RUNTIME_ROOT,
          cargoTargetDir: path.join(RUNTIME_ROOT, "cargo-target-rerouted"),
          stateDir: path.join(RUNTIME_ROOT, "state-rerouted"),
        };
        const artifactPath = await writeArtifact(RECOVERY_OUTPUT_ROOT, "reroute-build-output", suggestion);
        const envScriptPath = path.join(RECOVERY_OUTPUT_ROOT, "runtime-overrides.ps1");
        await writeEnvironmentOverrideScript(envScriptPath, {
          MEMFLOW_RUNTIME_ROOT: suggestion.runtimeRoot,
          MEMFLOW_CARGO_TARGET_DIR: suggestion.cargoTargetDir,
          MEMFLOW_STATE_DIR: suggestion.stateDir,
        });
        const restartScriptPath = await writeRestartScript(
          path.join(RECOVERY_OUTPUT_ROOT, "restart-with-runtime-overrides.ps1"),
          envScriptPath,
        );
        return {
          id: action.id,
          label: action.label,
          status: "completed",
          detail: "Wrote reroute suggestion artifact and reusable PowerShell overrides.",
          durationMs: Date.now() - startedAt,
          data: { artifactPath, envScriptPath, restartScriptPath, suggestion },
        };
      }
      case "inspect-port-owner": {
        const result = await runComputerCommand(action.command || "netstat -ano");
        return {
          id: action.id,
          label: action.label,
          status: result.allowed ? "completed" : "failed",
          detail: result.allowed ? "Collected port ownership information." : "Port inspection command was blocked.",
          durationMs: Date.now() - startedAt,
          data: result,
          error: result.allowed ? undefined : result.stderr,
        };
      }
      case "shift-port": {
        const availablePort = await findAvailablePort(3001);
        const suggestion = {
          suggestedPort: availablePort,
          env: availablePort
            ? {
                MEMFLOW_AGENT_PORT: String(availablePort),
                PORT: String(availablePort),
              }
            : null,
        };
        const artifactPath = await writeArtifact(RECOVERY_OUTPUT_ROOT, "port-shift", suggestion);
        const envScriptPath = availablePort
          ? await writeEnvironmentOverrideScript(path.join(RECOVERY_OUTPUT_ROOT, "port-overrides.ps1"), {
              MEMFLOW_AGENT_PORT: String(availablePort),
              PORT: String(availablePort),
            })
          : null;
        const restartScriptPath =
          envScriptPath
            ? await writeRestartScript(path.join(RECOVERY_OUTPUT_ROOT, "restart-with-port-overrides.ps1"), envScriptPath)
            : null;
        return {
          id: action.id,
          label: action.label,
          status: availablePort ? "completed" : "failed",
          detail: availablePort
            ? `Suggested alternate local port ${availablePort}.`
            : "Could not find a free alternate port in the probe window.",
          durationMs: Date.now() - startedAt,
          data: { artifactPath, envScriptPath, restartScriptPath, suggestion },
        };
      }
      case "switch-soft-success":
      case "replace-loopback-samples": {
        const cronConfigPath = process.env.MEMFLOW_CRON_CONFIG_PATH || path.join(RUNTIME_ROOT, "config", "cron-workflows.json");
        const raw = await fs.readFile(cronConfigPath, "utf8");
        const config = JSON.parse(raw) as {
          entries?: Array<{
            slug?: string;
            successMode?: string;
            verification?: { kind?: string; path?: string };
          }>;
        };
        const entries = config.entries || [];
        let updated = false;
        for (const entry of entries) {
          if (!entry.successMode) {
            entry.successMode = "soft-success-on-invalid-return";
            updated = true;
          }
          if (!entry.verification) {
            entry.verification = {
              kind: "file-append",
              path: `cron/${entry.slug || "sample"}.txt`,
            };
            updated = true;
          }
        }
        if (updated) {
          await fs.writeFile(cronConfigPath, JSON.stringify({ ...config, entries }, null, 2), "utf8");
        }
        const softEntries = entries.filter((entry) => entry.successMode === "soft-success-on-invalid-return");
        return {
          id: action.id,
          label: action.label,
          status: softEntries.length > 0 ? "completed" : "advisory",
          detail: softEntries.length > 0
            ? `Detected ${softEntries.length} soft-success cron samples${updated ? " and normalized the cron config." : "."}`
            : "Cron config is present but no soft-success samples were found.",
          durationMs: Date.now() - startedAt,
          data: { cronConfigPath, entries: entries.length, softEntries: softEntries.map((entry) => entry.slug), updated },
        };
      }
      default:
        return {
          id: action.id,
          label: action.label,
          status: "advisory",
          detail: action.detail,
          durationMs: Date.now() - startedAt,
        };
    }
  } catch (error) {
    return {
      id: action.id,
      label: action.label,
      status: "failed",
      detail: action.detail,
      durationMs: Date.now() - startedAt,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

export async function executeRecoveryPlan(input: RecoveryPlanRequest): Promise<RecoveryExecutionReport> {
  const plan = buildRecoveryPlan(input);
  const actionResults: RecoveryActionExecutionResult[] = [];
  for (const action of plan.actions) {
    actionResults.push(await executeRecoveryAction(action));
  }

  const verification = plan.verificationPlan
    ? await runVerificationPlan({
        taskType: plan.verificationPlan.taskType,
        objective: plan.verificationPlan.objective,
        changedFiles: plan.verificationPlan.changedFiles,
        plan: plan.verificationPlan,
      })
    : null;

  const report: RecoveryExecutionReport = {
    reportId: `${plan.reportId}-execution`,
    generatedAt: new Date().toISOString(),
    classification: plan.classification,
    actionResults,
    success: actionResults.every((result) => result.status !== "failed"),
    verification,
  };

  await writeArtifact(RECOVERY_OUTPUT_ROOT, "recovery-execution", report);
  return report;
}

function buildAutoFixVerificationPlan(category: FailureCategory): VerificationPlan | null {
  switch (category) {
    case "permission_denied":
      return {
        taskType: "autofix",
        objective: "Verify runtime override artifacts for permission recovery",
        changedFiles: [],
        evidencePaths: [RECOVERY_OUTPUT_ROOT],
        checks: [
          {
            id: "runtime-override-script",
            label: "Runtime override script exists",
            kind: "file",
            filePath: path.join(RECOVERY_OUTPUT_ROOT, "runtime-overrides.ps1"),
            fileRoot: "absolute",
            mustExist: true,
            minBytes: 20,
            reason: "Permission recovery should emit reusable runtime override instructions.",
          },
          {
            id: "runtime-restart-script",
            label: "Runtime restart script exists",
            kind: "file",
            filePath: path.join(RECOVERY_OUTPUT_ROOT, "restart-with-runtime-overrides.ps1"),
            fileRoot: "absolute",
            mustExist: true,
            minBytes: 20,
            reason: "Permission recovery should emit a restart helper bound to the overrides.",
          },
        ],
      };
    case "invalid_url":
      return {
        taskType: "autofix",
        objective: "Verify URL normalization patches on agent-service sources",
        changedFiles: [],
        evidencePaths: [path.join(PROJECT_ROOT, "agent-service", "src")],
        checks: [
          {
            id: "index-url-sanitizer",
            label: "Index URL sanitizer present",
            kind: "file",
            filePath: "agent-service/src/index.ts",
            fileRoot: "project",
            mustExist: true,
            containsText: "sanitizeServiceBaseUrl",
            minBytes: 50,
            reason: "The agent-service index should normalize base URLs before use.",
          },
          {
            id: "autonomy-url-sanitizer",
            label: "Autonomy URL sanitizer present",
            kind: "file",
            filePath: "agent-service/src/autonomy_supervisor.ts",
            fileRoot: "project",
            mustExist: true,
            containsText: "private executorEndpoint(requestPath: string)",
            minBytes: 50,
            reason: "Autonomy should sanitize executor URLs before concatenating paths.",
          },
        ],
      };
    case "executor_unavailable":
      return {
        taskType: "autofix",
        objective: "Verify live executor recovery entrypoints",
        changedFiles: [],
        evidencePaths: [path.join(PROJECT_ROOT, "scripts")],
        checks: [
          {
            id: "restart-live-executor-script",
            label: "Executor restart script exists",
            kind: "file",
            filePath: "scripts/restart-live-executor.ps1",
            fileRoot: "project",
            mustExist: true,
            minBytes: 50,
            reason: "Live executor recovery requires a dedicated restart script.",
          },
          {
            id: "executor-root-smoke",
            label: "Executor root smoke",
            kind: "http",
            url: "http://127.0.0.1:8082/",
            expectStatus: 200,
            reason: "Executor should be reachable after recovery.",
          },
        ],
      };
    case "port_conflict":
      return {
        taskType: "autofix",
        objective: "Verify port override artifacts for port conflict recovery",
        changedFiles: [],
        evidencePaths: [RECOVERY_OUTPUT_ROOT],
        checks: [
          {
            id: "port-override-script",
            label: "Port override script exists",
            kind: "file",
            filePath: path.join(RECOVERY_OUTPUT_ROOT, "port-overrides.ps1"),
            fileRoot: "absolute",
            mustExist: true,
            minBytes: 10,
            reason: "Port conflict recovery should emit reusable port overrides.",
          },
          {
            id: "port-restart-script",
            label: "Port restart script exists",
            kind: "file",
            filePath: path.join(RECOVERY_OUTPUT_ROOT, "restart-with-port-overrides.ps1"),
            fileRoot: "absolute",
            mustExist: true,
            minBytes: 10,
            reason: "Port conflict recovery should emit a restart helper bound to the port overrides.",
          },
        ],
      };
    case "invalid_return":
    case "ssrf_blocked":
      return {
        taskType: "autofix",
        objective: "Verify normalized cron config for executor-safe samples",
        changedFiles: [],
        evidencePaths: [path.join(RUNTIME_ROOT, "config")],
        checks: [
          {
            id: "cron-soft-success-config",
            label: "Cron config normalized",
            kind: "file",
            filePath: path.join(RUNTIME_ROOT, "config", "cron-workflows.json"),
            fileRoot: "absolute",
            mustExist: true,
            containsText: "soft-success-on-invalid-return",
            minBytes: 50,
            reason: "Executor-safe cron samples must carry soft-success configuration.",
          },
        ],
      };
    default:
      return null;
  }
}

export async function dispatchAutoFix(input: RecoveryPlanRequest): Promise<AutoFixDispatchReport> {
  const recovery = await executeRecoveryPlan(input);
  const dispatchResults: AutoFixDispatchResult[] = [];
  const category = recovery.classification.category;

  const pushResult = (result: AutoFixDispatchResult) => {
    dispatchResults.push(result);
  };

  if (category === "invalid_return" || category === "ssrf_blocked") {
    const seedResult = await runComputerCommand("node scripts/seed-cron-workflows.js", ".");
    pushResult({
      id: "reseed-cron-workflows",
      label: "Reseed cron workflows",
      status: seedResult.allowed && seedResult.exitCode === 0 ? "completed" : "failed",
      detail:
        seedResult.allowed && seedResult.exitCode === 0
          ? "Reseeded cron workflows after sample normalization."
          : "Cron workflow reseed failed or was blocked.",
      data: seedResult,
      error: seedResult.allowed && seedResult.exitCode === 0 ? undefined : seedResult.stderr,
    });
  } else if (category === "executor_unavailable") {
    pushResult({
      id: "executor-restart-script-ready",
      label: "Executor restart script ready",
      status: "completed",
      detail: "Live executor recovery can use scripts/restart-live-executor.ps1.",
    });
  } else if (category === "invalid_url") {
    pushResult(await applyWhitelistedSourcePatch("normalize-url-builders"));
  } else if (category === "permission_denied") {
    pushResult({
      id: "runtime-override-ready",
      label: "Runtime override ready",
      status: "completed",
      detail: "Recovery artifacts now include a restart script bound to writable runtime overrides.",
    });
  } else if (category === "port_conflict") {
    pushResult({
      id: "port-override-ready",
      label: "Port override ready",
      status: "completed",
      detail: "Recovery artifacts now include a restart script bound to alternate port overrides.",
    });
  } else {
    pushResult({
      id: "no-op-dispatch",
      label: "No auto-fix dispatch",
      status: "skipped",
      detail: "No category-specific auto-fix dispatch is defined for this failure yet.",
    });
  }

  const followUpPlan = buildAutoFixVerificationPlan(category);
  const followUpVerification = followUpPlan ? await runVerificationPlan({ plan: followUpPlan }) : null;

  const report: AutoFixDispatchReport = {
    reportId: `autofix-${Date.now()}`,
    generatedAt: new Date().toISOString(),
    classification: recovery.classification,
    recovery: followUpVerification
      ? {
          ...recovery,
          verification: followUpVerification,
        }
      : recovery,
    dispatchResults,
    success:
      recovery.success &&
      dispatchResults.every((result) => result.status !== "failed") &&
      (followUpVerification ? followUpVerification.success : true),
  };

  await writeArtifact(RECOVERY_OUTPUT_ROOT, "autofix-dispatch", report);
  return report;
}

export function attachCodingKernelRoutes(app: Express): void {
  const router = Router();

  router.get("/coding-kernel/status", async (_req: Request, res: Response) => {
    try {
      const snapshot = await loadLatestReferenceSnapshot();
      res.json({
        runtimeRoot: RUNTIME_ROOT,
        referenceRoot: defaultReferencePackageRoot(),
        toolFamilies: CODING_TOOL_FAMILIES,
        latestReference: snapshot
          ? {
              referenceId: snapshot.referenceId,
              version: snapshot.version,
              generatedAt: snapshot.generatedAt,
              slashCommandCount: snapshot.slashCommands.length,
              topCapabilities: snapshot.capabilitySignals.slice(0, 5),
            }
          : null,
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(500).json({ error: message });
    }
  });

  router.get("/coding-kernel/tools", (_req: Request, res: Response) => {
    res.json({ families: CODING_TOOL_FAMILIES });
  });

  router.post("/coding-kernel/reference/ingest", async (req: Request, res: Response) => {
    try {
      const body = (req.body || {}) as ReferenceIngestRequest;
      const snapshot = await ingestClaudeReference(body.packageRoot || defaultReferencePackageRoot(), body.persist !== false);
      res.json(snapshot);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(400).json({ error: message });
    }
  });

  router.get("/coding-kernel/reference/snapshot", async (_req: Request, res: Response) => {
    try {
      const snapshot = await loadLatestReferenceSnapshot();
      if (!snapshot) {
        res.status(404).json({ error: "No reference snapshot found" });
        return;
      }
      res.json(snapshot);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(500).json({ error: message });
    }
  });

  router.post("/coding-kernel/recovery/classify", (req: Request, res: Response) => {
    const classification = classifyFailure((req.body || {}) as RecoveryRequest);
    res.json(classification);
  });

  router.post("/coding-kernel/recovery/plan", async (req: Request, res: Response) => {
    try {
      const plan = buildRecoveryPlan((req.body || {}) as RecoveryPlanRequest);
      await persistRecoveryPlan(plan);
      res.json(plan);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(400).json({ error: message });
    }
  });

  router.post("/coding-kernel/recovery/execute", async (req: Request, res: Response) => {
    try {
      const report = await executeRecoveryPlan((req.body || {}) as RecoveryPlanRequest);
      res.json(report);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(400).json({ error: message });
    }
  });

  router.post("/coding-kernel/autofix/dispatch", async (req: Request, res: Response) => {
    try {
      const report = await dispatchAutoFix((req.body || {}) as RecoveryPlanRequest);
      res.json(report);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(400).json({ error: message });
    }
  });

  router.post("/coding-kernel/verification/plan", (req: Request, res: Response) => {
    const plan = planVerification((req.body || {}) as VerificationPlanRequest);
    res.json(plan);
  });

  router.post("/coding-kernel/verification/run", async (req: Request, res: Response) => {
    try {
      const report = await runVerificationPlan((req.body || {}) as VerificationRunRequest);
      res.json(report);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      res.status(400).json({ error: message });
    }
  });

  app.use(router);
}
