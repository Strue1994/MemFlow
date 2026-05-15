/**
 * Observability — OpenTelemetry-aligned tracing for agent executions
 *
 * Provides:
 * - Per-session trace spans (start → tool calls → complete)
 * - Agent execution timing
 * - Tool call performance
 * - /traces endpoint for dashboard
 *
 * Compatible with OTEL format for future collector export.
 */

import * as fs from "node:fs";
import * as path from "node:path";

// ---- Types ----

export interface TraceSpan {
  id: string;
  sessionId: string;
  parentId: string | null;
  name: string;
  type: "agent" | "tool" | "llm" | "memory" | "workflow";
  status: "ok" | "error";
  startedAt: string;
  durationMs: number;
  input: string;
  output: string;
  error?: string;
  metadata: Record<string, unknown>;
}

export interface TraceSummary {
  sessionId: string;
  totalDurationMs: number;
  spanCount: number;
  errorCount: number;
  spans: TraceSpan[];
}

// ---- Storage ----

function getTraceDir(): string {
  const root = process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(process.cwd(), "..", ".memflow-runtime");
  return path.resolve(root, "traces");
}

function getSessionPath(sessionId: string): string {
  return path.resolve(getTraceDir(), `${sessionId}.json`);
}

// ---- Trace Recorder ----

class TraceRecorder {
  private spans = new Map<string, TraceSpan[]>();
  private maxSessions = 100;

  /** Record a span */
  record(span: TraceSpan): void {
    const session = this.spans.get(span.sessionId) || [];
    session.push(span);
    this.spans.set(span.sessionId, session);

    // Persist after every 5 spans
    if (session.length % 5 === 0) {
      this.flush(span.sessionId);
    }

    // Prune oldest sessions
    if (this.spans.size > this.maxSessions) {
      const oldest = [...this.spans.keys()].slice(0, this.spans.size - this.maxSessions);
      for (const id of oldest) {
        this.spans.delete(id);
        try { fs.unlinkSync(getSessionPath(id)); } catch { /* ignore */ }
      }
    }
  }

  /** Flush spans for a session to disk */
  flush(sessionId: string): void {
    const spans = this.spans.get(sessionId);
    if (!spans || spans.length === 0) return;

    try {
      const dir = getTraceDir();
      if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(getSessionPath(sessionId), JSON.stringify(this.getSummary(sessionId), null, 2), "utf-8");
    } catch { /* best-effort */ }
  }

  /** Get trace summary for a session */
  getSummary(sessionId: string): TraceSummary | null {
    const spans = this.spans.get(sessionId);
    if (!spans || spans.length === 0) return null;

    const errorCount = spans.filter((s) => s.status === "error").length;
    const totalDuration = spans.reduce((a, b) => a + b.durationMs, 0);

    return {
      sessionId,
      totalDurationMs: totalDuration,
      spanCount: spans.length,
      errorCount,
      spans,
    };
  }

  /** List all tracked sessions */
  listSessions(): string[] {
    return [...this.spans.keys()];
  }

  /** Create a span */
  createSpan(params: {
    sessionId: string;
    parentId?: string;
    name: string;
    type: TraceSpan["type"];
    input?: string;
    metadata?: Record<string, unknown>;
  }): TraceSpan {
    const now = new Date().toISOString();
    return {
      id: `span_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      sessionId: params.sessionId,
      parentId: params.parentId || null,
      name: params.name,
      type: params.type,
      status: "ok",
      startedAt: now,
      durationMs: 0,
      input: params.input || "",
      output: "",
      metadata: params.metadata || {},
    };
  }

  /** Complete a span with timing + result */
  completeSpan(span: TraceSpan, output: string, error?: string): void {
    const startTime = new Date(span.startedAt).getTime();
    span.durationMs = Date.now() - startTime;
    span.output = output.slice(0, 2000);
    span.status = error ? "error" : "ok";
    if (error) span.error = error.slice(0, 500);
    this.record(span);
  }
}

export const globalTracer = new TraceRecorder();

// ---- Simple span helper for agent executions ----

export function traceAgent(sessionId: string, text: string): TraceSpan {
  return globalTracer.createSpan({
    sessionId,
    name: "agent.execute",
    type: "agent",
    input: text,
  });
}

export function traceTool(sessionId: string, parentId: string, toolName: string, input: string): TraceSpan {
  return globalTracer.createSpan({
    sessionId,
    parentId,
    name: `tool.${toolName}`,
    type: "tool",
    input,
  });
}
