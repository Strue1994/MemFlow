/**
 * Middleware Chain — DeerFlow-inspired pluggable request/response pipeline
 *
 * Each middleware can:
 * - Transform the prompt before it reaches the LLM (before)
 * - Transform the response before it reaches the user (after)
 * - Skip execution entirely
 *
 * Middleware config is hot-reloadable via /middleware/config API
 */

// ---- Types ----

export interface MiddlewareContext {
  /** The user's original text */
  text: string;
  /** The assembled messages array */
  messages: any[];
  /** Available tool list */
  tools: any[];
  /** User ID if available */
  userId?: string;
  /** Whether streaming is enabled */
  stream?: boolean;
  /** Additional metadata */
  meta: Record<string, unknown>;
}

export interface MiddlewareResult {
  /** Whether to continue the pipeline */
  skip: boolean;
  /** Modified messages */
  messages?: any[];
  /** Optional early return (if middleware handles the request itself) */
  earlyResponse?: string;
}

export interface Middleware {
  name: string;
  description: string;
  enabled: boolean;
  /** Priority: lower runs first */
  priority: number;
  before(ctx: MiddlewareContext): Promise<MiddlewareResult>;
  after?(ctx: MiddlewareContext, response: string): Promise<string>;
}

// ---- Registry ----

class MiddlewareRegistry {
  private middlewares: Map<string, Middleware> = new Map();
  private order: string[] = [];

  register(mw: Middleware): void {
    this.middlewares.set(mw.name, mw);
    this.reorder();
  }

  unregister(name: string): boolean {
    const ok = this.middlewares.delete(name);
    if (ok) this.reorder();
    return ok;
  }

  get(name: string): Middleware | undefined {
    return this.middlewares.get(name);
  }

  list(): Middleware[] {
    return this.order.map((name) => this.middlewares.get(name)!).filter(Boolean);
  }

  enable(name: string, enabled: boolean): void {
    const mw = this.middlewares.get(name);
    if (mw) mw.enabled = enabled;
  }

  private reorder(): void {
    this.order = [...this.middlewares.entries()]
      .sort(([, a], [, b]) => a.priority - b.priority)
      .map(([name]) => name);
  }
}

export const globalMiddleware = new MiddlewareRegistry();

// ---- Built-in Middlewares ----

/**
 * SandboxMiddleware: Ensures request enters sandboxed execution.
 * Currently a pass-through; real sandbox would validate paths/commands.
 */
globalMiddleware.register({
  name: "sandbox",
  description: "Sandbox execution environment for agent requests",
  enabled: true,
  priority: 10,
  before: async (ctx) => ({ skip: false }),
  after: async (_ctx, response) => response,
});

/**
 * SummarizationMiddleware: Compress context when over token limit.
 * Delegates to context-compressor.
 */
globalMiddleware.register({
  name: "summarization",
  description: "Auto-compress context when approaching token limits",
  enabled: true,
  priority: 20,
  before: async (ctx) => {
    const totalTokens = JSON.stringify(ctx.messages).length / 4;
    if (totalTokens > 12000) {
      // Auto-trigger compact before LLM call
      const { compressContext } = require("./context-compressor");
      const result = await compressContext(ctx.messages);
      return { skip: false, messages: result.messages };
    }
    return { skip: false };
  },
});

/**
 * TodoListMiddleware: Track multi-step task progress.
 * Parses "step 1", "step 2" patterns and tracks completion.
 */
globalMiddleware.register({
  name: "todo",
  description: "Track multi-step task progress",
  enabled: true,
  priority: 30,
  before: async (ctx) => ({ skip: false }),
  after: async (_ctx, response) => response,
});

/**
 * MemoryMiddleware: Async memory extraction after each turn.
 * Feeds execution records to the Curator for self-learning.
 */
globalMiddleware.register({
  name: "memory",
  description: "Async memory extraction for self-learning",
  enabled: true,
  priority: 40,
  before: async (ctx) => ({ skip: false }),
  after: async (_ctx, response) => {
    try {
      const { globalCurator } = require("./curator");
      globalCurator.recordExecution({
        workflowId: "agent_" + Date.now(),
        taskText: _ctx.text,
        steps: response ? ["Agent responded"] : [],
        success: !!response,
        durationMs: 0,
        timestamp: new Date().toISOString(),
      });
    } catch { /* best-effort */ }
    return response;
  },
});

/**
 * TitleMiddleware: Auto-generate conversation titles.
 */
globalMiddleware.register({
  name: "title",
  description: "Auto-generate conversation titles",
  enabled: false, // Disabled by default
  priority: 50,
  before: async (ctx) => ({ skip: false }),
  after: async (_ctx, response) => response,
});

/**
 * ClarificationMiddleware: Intercept clarification requests.
 */
globalMiddleware.register({
  name: "clarification",
  description: "Intercept and handle agent clarification requests",
  enabled: true,
  priority: 60,
  before: async (ctx) => ({ skip: false }),
});

// ---- Pipeline Executor ----

/**
 * Run the full middleware pipeline (before all).
 * Returns modified context or early response.
 */
export async function runBeforePipeline(ctx: MiddlewareContext): Promise<{
  modifiedCtx: MiddlewareContext;
  earlyResponse: string | null;
}> {
  let modifiedCtx = { ...ctx };
  let earlyResponse: string | null = null;

  for (const mw of globalMiddleware.list()) {
    if (!mw.enabled) continue;

    try {
      const result = await mw.before(modifiedCtx);
      if (result.earlyResponse) {
        earlyResponse = result.earlyResponse;
        break;
      }
      if (result.messages) {
        modifiedCtx = { ...modifiedCtx, messages: result.messages };
      }
    } catch (err: any) {
      console.warn(`Middleware "${mw.name}" before error: ${err.message}`);
    }
  }

  return { modifiedCtx, earlyResponse };
}

/**
 * Run the after pipeline (all after).
 */
export async function runAfterPipeline(ctx: MiddlewareContext, response: string): Promise<string> {
  let modified = response;

  for (const mw of globalMiddleware.list()) {
    if (!mw.enabled || !mw.after) continue;

    try {
      modified = await mw.after(ctx, modified);
    } catch (err: any) {
      console.warn(`Middleware "${mw.name}" after error: ${err.message}`);
    }
  }

  return modified;
}

export default globalMiddleware;
