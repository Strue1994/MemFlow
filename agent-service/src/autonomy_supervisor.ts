import { Express, Router } from "express";
import fs from "fs";
import path from "path";
import { dispatchAutoFix } from "./coding_kernel";

export interface WorkflowSummary {
  id: string;
  name?: string;
  version?: number;
  nodes?: number;
}

interface OptimizationRecommendation {
  name: string;
  current?: number | string;
  recommended?: number | string;
  impact?: string;
  reason?: string;
}

interface OptimizationResponse {
  params?: OptimizationRecommendation[];
  estimated_speedup?: number;
  estimated_accuracy?: number;
  estimated_cost_savings?: number;
}

interface TaskRecord {
  id?: string;
  task_id?: string;
  workflow_id: string;
  status: string;
}

interface SupervisorDeps {
  executorUrl: string;
  buildExecutorHeaders(includeJson?: boolean): Record<string, string>;
  listWorkflows(): Promise<WorkflowSummary[]>;
  remember(content: string, metadata?: object): Promise<void>;
  recall(query: string, k?: number): Promise<string[]>;
}

interface ReflectionEntry {
  at: string;
  kind: "observe" | "reflect" | "act" | "error";
  message: string;
  data?: unknown;
}

interface WorkflowHealth {
  workflowId: string;
  successRate: number;
  avgDurationMs: number;
  totalRuns: number;
  failures: number;
  latestError: string | null;
}

interface CronVerification {
  kind?: string;
  path?: string;
}

interface CronEntry {
  slug: string;
  name: string;
  workflowId: string;
  enabled?: boolean;
  successMode?: string;
  verification?: CronVerification;
}

interface CronConfig {
  generatedAt?: string;
  entries?: CronEntry[];
}

interface CronRunnerState {
  lastRunAt?: Record<string, string>;
  lastSuccessAt?: Record<string, string>;
  lastSoftSuccessAt?: Record<string, string>;
  lastResult?: Record<string, { status?: string; at?: string; message?: string }>;
}

interface AutonomyStatus {
  enabled: boolean;
  running: boolean;
  objective: string;
  intervalSeconds: number;
  lastTickAt: string | null;
  nextTickAt: string | null;
  lastAction: string | null;
  lastError: string | null;
  recent: ReflectionEntry[];
}

interface StartOptions {
  intervalSeconds?: number;
  objective?: string;
}

interface UpgradeSuggestion {
  id: string;
  source_unit_id: string;
  new_content: string;
  confidence: number;
  reason: string;
  auto_merge: boolean;
  status: string;
}

const DEFAULT_INTERVAL_SECONDS = Number(process.env.AUTONOMY_INTERVAL_SECONDS || "120");
const DEFAULT_OBJECTIVE =
  process.env.AUTONOMY_OBJECTIVE ||
  "Continuously improve workflow reliability by observing execution health, creating remediation tasks, and applying safe tuning actions.";
const SAFE_TUNING_KEYS = new Set(["max_retries", "timeout_ms"]);
const RUNTIME_ROOT =
  process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(__dirname, "..", "..", ".memflow-runtime");
const CRON_CONFIG_PATH =
  process.env.MEMFLOW_CRON_CONFIG_PATH || path.join(RUNTIME_ROOT, "config", "cron-workflows.json");
const CRON_STATE_PATH = path.join(RUNTIME_ROOT, "state", "cron-runner-state.json");
const RETURNLESS_SUCCESS_MESSAGE = "Invalid return: no value to return";

class AutonomySupervisor {
  private timer: NodeJS.Timeout | null = null;
  private tickInFlight = false;
  private status: AutonomyStatus = {
    enabled: false,
    running: false,
    objective: DEFAULT_OBJECTIVE,
    intervalSeconds: DEFAULT_INTERVAL_SECONDS,
    lastTickAt: null,
    nextTickAt: null,
    lastAction: null,
    lastError: null,
    recent: [],
  };

  constructor(private readonly deps: SupervisorDeps) {}

  getStatus(): AutonomyStatus {
    return { ...this.status, recent: [...this.status.recent] };
  }

  start(options: StartOptions = {}): AutonomyStatus {
    if (options.intervalSeconds && options.intervalSeconds > 0) {
      this.status.intervalSeconds = options.intervalSeconds;
    }
    if (options.objective?.trim()) {
      this.status.objective = options.objective.trim();
    }

    this.status.enabled = true;
    this.status.running = true;
    this.status.lastError = null;
    this.record("reflect", "Autonomy supervisor started", {
      objective: this.status.objective,
      intervalSeconds: this.status.intervalSeconds,
    });
    this.scheduleNextTick(0);
    return this.getStatus();
  }

  stop(): AutonomyStatus {
    this.status.enabled = false;
    this.status.running = false;
    this.status.nextTickAt = null;
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    this.record("reflect", "Autonomy supervisor stopped");
    return this.getStatus();
  }

  async tickNow(): Promise<AutonomyStatus> {
    await this.tick();
    return this.getStatus();
  }

  private scheduleNextTick(delayMs?: number): void {
    if (!this.status.enabled) {
      return;
    }
    if (this.timer) {
      clearTimeout(this.timer);
    }
    const waitMs = delayMs ?? this.status.intervalSeconds * 1000;
    this.status.nextTickAt = new Date(Date.now() + waitMs).toISOString();
    this.timer = setTimeout(() => {
      void this.tick();
    }, waitMs);
  }

  private record(kind: ReflectionEntry["kind"], message: string, data?: unknown): void {
    this.status.recent.unshift({
      at: new Date().toISOString(),
      kind,
      message,
      data,
    });
    this.status.recent = this.status.recent.slice(0, 20);
    if (kind === "act") {
      this.status.lastAction = message;
    }
    if (kind === "error") {
      this.status.lastError = message;
    }
  }

  private async tick(): Promise<void> {
    if (!this.status.enabled || this.tickInFlight) {
      return;
    }
    this.tickInFlight = true;
    this.status.lastTickAt = new Date().toISOString();
    this.status.nextTickAt = null;

    try {
      const workflows = await this.deps.listWorkflows();
      const cronEntries = this.getEnabledCronEntries();
      const candidateWorkflows =
        cronEntries.length > 0
          ? workflows.filter((workflow) => cronEntries.some((entry) => entry.workflowId === workflow.id))
          : workflows;
      this.record("observe", "Observed workflow inventory", {
        count: workflows.length,
        activeCount: candidateWorkflows.length,
      });

      const objectiveMemories = await this.deps.recall(this.status.objective, 3);
      if (objectiveMemories.length > 0) {
        this.record("reflect", "Recalled prior autonomy memories", {
          objective: this.status.objective,
          memories: objectiveMemories,
        });
      }

      if (candidateWorkflows.length === 0) {
        this.record("reflect", "No workflows available; autonomy loop is idle");
        await this.deps.remember("Autonomy loop observed no workflows and stayed idle.", {
          objective: this.status.objective,
        });
        return;
      }

      const healthSnapshots = await Promise.all(
        candidateWorkflows
          .slice(0, 10)
          .map((workflow) =>
            this.computeWorkflowHealth(
              workflow.id,
              cronEntries.find((entry) => entry.workflowId === workflow.id),
            ),
          ),
      );
      const focus = healthSnapshots.sort(this.compareHealth)[0] || null;

      if (!focus) {
        this.record("reflect", "No health snapshot available; skipping action");
        return;
      }

      this.record("reflect", "Selected workflow focus", focus);
      const workflowMemories = await this.deps.recall(`workflow ${focus.workflowId} tuning`, 5);
      if (workflowMemories.length > 0) {
        this.record("reflect", "Recalled workflow-specific memories", {
          workflowId: focus.workflowId,
          memories: workflowMemories,
        });
      }

      if (focus.successRate === 0 && focus.failures >= 3 && focus.latestError) {
        const recovery = await dispatchAutoFix({
          error: focus.latestError,
          objective: `Recover failing workflow ${focus.workflowId}`,
        });
        this.record("act", `Executed coding recovery for ${focus.workflowId}`, {
          category: recovery.classification.category,
          actionResults: recovery.recovery.actionResults,
          dispatchResults: recovery.dispatchResults,
          verificationSuccess: recovery.recovery.verification?.success ?? null,
        });
        await this.ensureEvidenceTask(focus.workflowId, "recover", {
          focus,
          recovery,
        });
        await this.deps.remember(
          `Autonomy supervisor executed coding recovery for workflow ${focus.workflowId} after repeated failures: ${recovery.classification.category}`,
          {
            workflowId: focus.workflowId,
            latestError: focus.latestError,
            category: recovery.classification.category,
            dispatchResults: recovery.dispatchResults,
          },
        );
        await this.processSafeUpgradeSuggestions();
        return;
      }

      const optimize = await this.postJson<OptimizationResponse>("/optimize", {
        workflow_id: focus.workflowId,
      });

      const safeParams = this.extractSafeTuning(optimize.params || []);
      if (workflowMemories.some((memory) => this.memoryContainsAction(memory, focus.workflowId, safeParams))) {
        this.record("reflect", "Skipping repeated safe tuning due to matching prior reflection", {
          workflowId: focus.workflowId,
          safeParams,
        });
        await this.processSafeUpgradeSuggestions();
        return;
      }

      if (Object.keys(safeParams).length === 0) {
        this.record("reflect", "No safe tuning recommendation found", {
          workflowId: focus.workflowId,
          recommendations: optimize.params || [],
        });
        await this.ensureEvidenceTask(focus.workflowId, "observe", {
          focus,
          recommendations: optimize.params || [],
          decision: "no-safe-action",
        });
        return;
      }

      const tuningResult = await this.postJson<{ success?: boolean; message?: string }>("/apply-tuning", {
        workflow_id: focus.workflowId,
        params: safeParams,
      });
      await this.ensureEvidenceTask(focus.workflowId, "act", {
        focus,
        safeParams,
        tuningResult,
      });

      this.record("act", `Applied safe tuning to ${focus.workflowId}`, {
        params: safeParams,
        result: tuningResult,
      });
      await this.deps.remember(
        `Autonomy supervisor applied safe tuning to workflow ${focus.workflowId}: ${JSON.stringify(safeParams)}`,
        {
          workflowId: focus.workflowId,
          safeParams,
          focus,
        },
      );

      await this.processSafeUpgradeSuggestions();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      this.record("error", message);
      if (message.includes("Rate limit exceeded")) {
        this.scheduleNextTick(Math.max(this.status.intervalSeconds, 300) * 1000);
        this.tickInFlight = false;
        return;
      }
    } finally {
      this.tickInFlight = false;
      this.scheduleNextTick();
    }
  }

  private compareHealth(a: WorkflowHealth, b: WorkflowHealth): number {
    const aScore = (1 - a.successRate) * 1000 + a.failures * 10 + a.avgDurationMs / 1000;
    const bScore = (1 - b.successRate) * 1000 + b.failures * 10 + b.avgDurationMs / 1000;
    return bScore - aScore;
  }

  private async computeWorkflowHealth(workflowId: string, cronEntry?: CronEntry): Promise<WorkflowHealth> {
    const logs = await this.getJson<Array<{ error?: string | null; duration_ms?: number }>>(
      `/workflow/${encodeURIComponent(workflowId)}/logs?limit=20`,
    );
    const cronState = cronEntry ? this.readCronRunnerState() : null;

    const totalRuns = logs.length;
    const failures = logs.filter((log) => !this.isEffectiveSuccess(log, cronEntry, cronState)).length;
    const successRate = totalRuns > 0 ? (totalRuns - failures) / totalRuns : 1;
    const avgDurationMs =
      totalRuns > 0
        ? logs.reduce((sum, log) => sum + Number(log.duration_ms || 0), 0) / totalRuns
        : 0;
    const latestError = logs.find((log) => typeof log.error === "string" && log.error.trim().length > 0)?.error || null;

    return {
      workflowId,
      successRate,
      avgDurationMs,
      totalRuns,
      failures,
      latestError,
    };
  }

  private isEffectiveSuccess(
    log: { error?: string | null },
    cronEntry?: CronEntry,
    cronState?: CronRunnerState | null,
  ): boolean {
    if (!log.error) {
      return true;
    }
    if (!cronEntry || !cronState) {
      return false;
    }
    if (cronEntry.successMode !== "soft-success-on-invalid-return") {
      return false;
    }
    if (!log.error.includes(RETURNLESS_SUCCESS_MESSAGE)) {
      return false;
    }
    const status = cronState.lastResult?.[cronEntry.slug]?.status;
    return status === "soft-success" || status === "success";
  }

  private getEnabledCronEntries(): CronEntry[] {
    const config = this.readJsonFile<CronConfig>(CRON_CONFIG_PATH, { entries: [] });
    return (config.entries || []).filter((entry) => entry.enabled !== false && Boolean(entry.workflowId));
  }

  private readCronRunnerState(): CronRunnerState {
    return this.readJsonFile<CronRunnerState>(CRON_STATE_PATH, {
      lastRunAt: {},
      lastSuccessAt: {},
      lastSoftSuccessAt: {},
      lastResult: {},
    });
  }

  private readJsonFile<T>(filePath: string, fallback: T): T {
    try {
      return JSON.parse(fs.readFileSync(filePath, "utf8")) as T;
    } catch {
      return fallback;
    }
  }

  private extractSafeTuning(recommendations: OptimizationRecommendation[]): Record<string, number | string> {
    const params: Record<string, number | string> = {};

    for (const recommendation of recommendations) {
      if (!SAFE_TUNING_KEYS.has(recommendation.name)) {
        continue;
      }
      if (recommendation.recommended === undefined || recommendation.recommended === null) {
        continue;
      }
      params[recommendation.name] = recommendation.recommended;
    }

    return params;
  }

  private memoryContainsAction(memory: string, workflowId: string, params: Record<string, number | string>): boolean {
    if (!memory.includes(workflowId)) {
      return false;
    }
    const serialized = JSON.stringify(params);
    return serialized !== "{}" && memory.includes(serialized);
  }

  private async processSafeUpgradeSuggestions(): Promise<void> {
    const payload = await this.getJson<{ suggestions?: UpgradeSuggestion[] }>("/knowledge/upgrade/suggestions");
    const suggestions = payload.suggestions || [];
    const pendingLowRisk = suggestions.filter((suggestion) => this.isLowRiskUpgrade(suggestion));

    for (const suggestion of pendingLowRisk) {
      await this.postJson(`/knowledge/upgrade/suggestions/${encodeURIComponent(suggestion.id)}/approve`, {});
      await this.postJson(`/knowledge/upgrade/${encodeURIComponent(suggestion.id)}/merge`, {});
      this.record("act", `Auto-approved low-risk upgrade ${suggestion.id}`, {
        sourceUnitId: suggestion.source_unit_id,
        confidence: suggestion.confidence,
      });
      await this.deps.remember(
        `Autonomy supervisor auto-approved low-risk upgrade ${suggestion.id} for knowledge unit ${suggestion.source_unit_id}.`,
        {
          suggestionId: suggestion.id,
          sourceUnitId: suggestion.source_unit_id,
          confidence: suggestion.confidence,
        },
      );
    }
  }

  private isLowRiskUpgrade(suggestion: UpgradeSuggestion): boolean {
    if (suggestion.status !== "Pending") {
      return false;
    }
    if (!(suggestion.auto_merge || suggestion.confidence >= 0.92)) {
      return false;
    }
    if (suggestion.new_content.length > 4000) {
      return false;
    }
    return !/(delete|drop|truncate|credential|secret|token|password|exec|script|shell|unsafe)/i.test(
      `${suggestion.reason}\n${suggestion.new_content}`,
    );
  }

  private async ensureEvidenceTask(workflowId: string, checkpoint: string, evidence: unknown): Promise<void> {
    const tasks = await this.getJson<TaskRecord[]>(
      `/tasks?workflow_id=${encodeURIComponent(workflowId)}&limit=20`,
    );
    const active = tasks.find((task) => ["created", "running", "review", "blocked"].includes(task.status));
    const taskId =
      active?.id ||
      active?.task_id ||
      (
        await this.postJson<{ task_id: string }>("/tasks", {
          workflow_id: workflowId,
          owner: "autonomy-supervisor",
          checkpoint,
        })
      ).task_id;

    await this.postJson(`/tasks/${encodeURIComponent(taskId)}/evidence`, {
      checkpoint,
      owner: "autonomy-supervisor",
      evidence,
    });
  }

  private async getJson<T>(path: string): Promise<T> {
    const response = await fetch(this.executorEndpoint(path), {
      headers: this.deps.buildExecutorHeaders(),
    });

    if (!response.ok) {
      throw new Error(await this.readError(response));
    }

    return (await response.json()) as T;
  }

  private async postJson<T>(path: string, body: object): Promise<T> {
    const response = await fetch(this.executorEndpoint(path), {
      method: "POST",
      headers: this.deps.buildExecutorHeaders(true),
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      throw new Error(await this.readError(response));
    }

    return (await response.json()) as T;
  }

  private async readError(response: Response): Promise<string> {
    try {
      const payload = (await response.json()) as { error?: string };
      return payload.error || `HTTP ${response.status}`;
    } catch {
      return `HTTP ${response.status}`;
    }
  }

  private executorEndpoint(requestPath: string): string {
    const base = this.deps.executorUrl.trim().replace(/\s+/g, "").replace(/\/+$/, "");
    return `${base}${requestPath}`;
  }
}

export function attachAutonomyRoutes(app: Express, deps: SupervisorDeps): AutonomySupervisor {
  const supervisor = new AutonomySupervisor(deps);
  const router = Router();

  router.get("/status", (_req, res) => {
    res.json(supervisor.getStatus());
  });

  router.post("/start", async (req, res) => {
    const status = supervisor.start({
      intervalSeconds: req.body?.intervalSeconds,
      objective: req.body?.objective,
    });
    res.json(status);
  });

  router.post("/stop", (_req, res) => {
    res.json(supervisor.stop());
  });

  router.post("/tick", async (_req, res) => {
    const status = await supervisor.tickNow();
    res.json(status);
  });

  app.use("/autonomy", router);

  if (String(process.env.AUTONOMY_ENABLED || "").toLowerCase() === "true") {
    supervisor.start();
  }

  return supervisor;
}
