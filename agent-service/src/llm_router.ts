/**
 * LLM Router — Hermes/OpenClaw-aligned smart provider routing
 *
 * Features:
 * - Task complexity classification (simple → medium → complex → expert)
 * - 4-tier provider selection with cost awareness
 * - Multi-provider key discovery from env vars
 * - Per-session cost tracking
 * - Automatic tier escalation on repeated failures
 */

// ---- Types ----

import { type ComplexityTier } from "./types";

export type { ComplexityTier };

export interface RouterProvider {
  id: string;          // e.g. "openai", "anthropic", "groq"
  tier: ComplexityTier;
  model: string;
  baseUrl: string;
  apiKey: string;
  priority: number;    // lower = preferred within same tier
}

export interface RoutingDecision {
  tier: ComplexityTier;
  providers: RouterProvider[];
  reason: string;
}

export interface CostRecord {
  providerId: string;
  model: string;
  tokensIn: number;
  tokensOut: number;
  cost: number;
}

// Cost per 1M tokens (approximate USD, as of 2026-05)
const MODEL_COST: Record<string, { in: number; out: number }> = {
  // Tier 1 — cheap
  "llama-3.3-70b":       { in: 0.59,  out: 0.79  },
  "deepseek-v3":         { in: 0.27,  out: 1.10  },
  "gemini-2.0-flash":    { in: 0.10,  out: 0.40  },
  // Tier 2 — balanced
  "gpt-4o-mini":         { in: 0.15,  out: 0.60  },
  "claude-3.5-haiku":    { in: 0.80,  out: 4.00  },
  // Tier 3 — powerful
  "gpt-4o":              { in: 2.50,  out: 10.00 },
  "claude-sonnet-4":     { in: 3.00,  out: 15.00 },
  "gemini-2.0-pro":      { in: 1.25,  out: 5.00  },
  // Tier 4 — expert
  "o3-mini":             { in: 1.10,  out: 4.40  },
  "claude-opus-4":       { in: 15.00, out: 75.00 },
};

// ---- Provider registry ----

interface ProviderTemplate {
  id: string;
  tier: ComplexityTier;
  model: string;
  baseUrl: string;
  envKey: string;
  priority: number;
}

const PROVIDER_REGISTRY: ProviderTemplate[] = [
  // Tier 1: Cheap/Fast
  { id: "groq",        tier: "simple",  model: "llama-3.3-70b-versatile",    baseUrl: "https://api.groq.com/openai/v1",         envKey: "GROQ_API_KEY",          priority: 1 },
  { id: "deepseek",    tier: "simple",  model: "deepseek-chat",              baseUrl: "https://api.deepseek.com/v1",            envKey: "DEEPSEEK_API_KEY",       priority: 2 },
  { id: "gemini",      tier: "simple",  model: "gemini-2.0-flash",           baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai", envKey: "GEMINI_API_KEY", priority: 3 },

  // Tier 2: Balanced
  { id: "openai",      tier: "medium",  model: "gpt-4o-mini",                baseUrl: "https://api.openai.com/v1",              envKey: "OPENAI_API_KEY",         priority: 1 },
  { id: "anthropic",   tier: "medium",  model: "claude-3.5-haiku",           baseUrl: "https://api.anthropic.com/v1",           envKey: "ANTHROPIC_API_KEY",      priority: 2 },
  { id: "together",    tier: "medium",  model: "meta-llama/Llama-3.3-70B-Instruct-Turbo", baseUrl: "https://api.together.xyz/v1", envKey: "TOGETHER_API_KEY", priority: 3 },

  // Tier 3: Powerful
  { id: "openai",      tier: "complex", model: "gpt-4o",                     baseUrl: "https://api.openai.com/v1",              envKey: "OPENAI_API_KEY",         priority: 1 },
  { id: "anthropic",   tier: "complex", model: "claude-sonnet-4-20250514",   baseUrl: "https://api.anthropic.com/v1",           envKey: "ANTHROPIC_API_KEY",      priority: 2 },
  { id: "gemini",      tier: "complex", model: "gemini-2.0-pro",             baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai", envKey: "GEMINI_API_KEY", priority: 3 },

  // Tier 4: Expert
  { id: "openai",      tier: "expert",  model: "o3-mini",                    baseUrl: "https://api.openai.com/v1",              envKey: "OPENAI_API_KEY",         priority: 1 },
  { id: "anthropic",   tier: "expert",  model: "claude-opus-4-20250514",     baseUrl: "https://api.anthropic.com/v1",           envKey: "ANTHROPIC_API_KEY",      priority: 2 },
  { id: "deepseek",    tier: "expert",  model: "deepseek-reasoner",          baseUrl: "https://api.deepseek.com/v1",            envKey: "DEEPSEEK_API_KEY",       priority: 3 },

  // Extra cheap options (OpenAI-compatible)
  { id: "perplexity",  tier: "simple",  model: "sonar-pro",                  baseUrl: "https://api.perplexity.ai",              envKey: "PERPLEXITY_API_KEY",     priority: 4 },
  { id: "xai",         tier: "simple",  model: "grok-2-latest",              baseUrl: "https://api.x.ai/v1",                    envKey: "XAI_API_KEY",            priority: 5 },
  { id: "mistral",     tier: "medium",  model: "mistral-large-latest",       baseUrl: "https://api.mistral.ai/v1",              envKey: "MISTRAL_API_KEY",        priority: 4 },
];

// ---- Complexity Classifier ----

const COMPLEXITY_KEYWORDS: Record<ComplexityTier, RegExp[]> = {
  simple:  [ /^(hi|hello|hey|test)\b/i, /^\.{1,3}$/, /^\w{1,15}$/ ],
  medium:  [ /^(what|how|why|when|where|who)\b/i, /\byou are\b/i, /^(tell|explain|describe|show)\b/i ],
  complex: [ /(code|function|implement|refactor|debug|analyze|design|architect|optimize|migrate)/i,
             /\b(workflow|pipeline|api|schema|database|algorithm|compon)/i,
             /```/, /\n.*\{.*\n/, /multi-step|several step|multiple/i ],
  expert:  [ /(security|vulnerability|cryptograph|prove|theorem|optimize.*complex)/i,
             /\b(research|analyze.*deep|comprehensive|formal)/i,
             /\{[\s\S]{200,}\}/, /\n.{300,}/ ],
};

function classifyComplexity(text: string, toolCount: number): ComplexityTier {
  // Many tools available → likely complex task
  if (toolCount > 8) return "complex";
  if (toolCount > 15) return "expert";

  // Keyword matching (priority order: expert → complex → medium → simple)
  for (const tier of ["expert", "complex"] as ComplexityTier[]) {
    for (const re of COMPLEXITY_KEYWORDS[tier]) {
      if (re.test(text)) return tier;
    }
  }

  // Length-based heuristics
  const len = text.length;
  if (len > 2000) return "complex";
  if (len > 500) return "medium";

  // Short queries
  for (const re of COMPLEXITY_KEYWORDS.medium) {
    if (re.test(text)) return "medium";
  }
  if (len < 20) return "simple";

  return "medium";
}

// ---- Cost Tracker ----

class CostTracker {
  private records: CostRecord[] = [];
  private sessionStart: number;

  constructor() {
    this.sessionStart = Date.now();
  }

  record(providerId: string, model: string, tokensIn: number, tokensOut: number): void {
    const costs = MODEL_COST[model] || { in: 1.0, out: 4.0 };
    const cost = (tokensIn / 1_000_000) * costs.in + (tokensOut / 1_000_000) * costs.out;
    this.records.push({ providerId, model, tokensIn, tokensOut, cost });
  }

  getTotalCost(): number {
    return this.records.reduce((s, r) => s + r.cost, 0);
  }

  getStats() {
    const byProvider: Record<string, { calls: number; cost: number; tokensIn: number; tokensOut: number }> = {};
    for (const r of this.records) {
      if (!byProvider[r.providerId]) byProvider[r.providerId] = { calls: 0, cost: 0, tokensIn: 0, tokensOut: 0 };
      byProvider[r.providerId].calls++;
      byProvider[r.providerId].cost += r.cost;
      byProvider[r.providerId].tokensIn += r.tokensIn;
      byProvider[r.providerId].tokensOut += r.tokensOut;
    }
    return {
      sessionDurationMs: Date.now() - this.sessionStart,
      totalCost: this.getTotalCost(),
      totalCalls: this.records.length,
      byProvider,
    };
  }

  reset(): void {
    this.records = [];
    this.sessionStart = Date.now();
  }
}

// Global cost tracker (per agent-service instance)
export const globalCostTracker = new CostTracker();

// ---- Router ----

export interface RouterConfig {
  mode: "auto" | "manual";
  manualTier: ComplexityTier;
  escalation: boolean;   // auto-escalate on repeated failures
}

let routerConfig: RouterConfig = {
  mode: "auto",
  manualTier: "medium",
  escalation: true,
};

export function getRouterConfig(): RouterConfig {
  return { ...routerConfig };
}

export function setRouterConfig(cfg: Partial<RouterConfig>): RouterConfig {
  routerConfig = { ...routerConfig, ...cfg };
  return getRouterConfig();
}

/**
 * Discover available API keys from environment variables AND provider config file.
 * File-based config takes priority over env vars.
 */
function discoverAvailableKeys(): Record<string, string> {
  const keys: Record<string, string> = {};
  // 1. Env vars
  const vars = ["OPENAI_API_KEY", "ANTHROPIC_API_KEY", "GROQ_API_KEY", "DEEPSEEK_API_KEY",
    "GEMINI_API_KEY", "TOGETHER_API_KEY", "PERPLEXITY_API_KEY", "XAI_API_KEY", "MISTRAL_API_KEY"];
  for (const v of vars) {
    const val = process.env[v]?.trim();
    if (val) keys[v] = val;
  }
  // 2. File-based provider config (higher priority — overrides env)
  const ENV_KEY_MAP: Record<string, string> = {
    openai: "OPENAI_API_KEY",
    "openai-powerful": "OPENAI_API_KEY",
    "openai-expert": "OPENAI_API_KEY",
    anthropic: "ANTHROPIC_API_KEY",
    "anthropic-powerful": "ANTHROPIC_API_KEY",
    "anthropic-expert": "ANTHROPIC_API_KEY",
    groq: "GROQ_API_KEY",
    deepseek: "DEEPSEEK_API_KEY",
    "deepseek-expert": "DEEPSEEK_API_KEY",
    "gemini-flash": "GEMINI_API_KEY",
    "gemini-pro": "GEMINI_API_KEY",
    together: "TOGETHER_API_KEY",
    perplexity: "PERPLEXITY_API_KEY",
    xai: "XAI_API_KEY",
    mistral: "MISTRAL_API_KEY",
    ollama: "OLLAMA_API_KEY",
  };

  try {
    // Dynamic import to avoid circular deps at module level
    const { getEnabledProviders } = require("./provider-config");
    const fileProviders = getEnabledProviders() as any[];
    if (fileProviders?.length) {
      for (const p of fileProviders) {
        const envKey = ENV_KEY_MAP[p.id];
        if (envKey && p.apiKey) keys[envKey] = p.apiKey;
      }
    }
  } catch { /* provider-config may not be built yet */ }

  return keys;
}

/**
 * Select the best provider chain for a given task.
 */
export function selectProviders(text: string, toolCount: number, config?: Partial<RouterConfig>): RoutingDecision {
  const cfg = { ...routerConfig, ...config };
  const keys = discoverAvailableKeys();

  const tier = cfg.mode === "manual" ? cfg.manualTier : classifyComplexity(text, toolCount);

  // Gather matching providers for this tier
  const candidates = PROVIDER_REGISTRY
    .filter((p) => p.tier === tier && keys[p.envKey])
    .sort((a, b) => a.priority - b.priority);

  // If no providers in this tier, fall back to the next available tier
  let actualTier = tier;
  let providers: RouterProvider[] = candidates.map((p) => ({
    id: p.id, tier: p.tier, model: p.model, baseUrl: p.baseUrl,
    apiKey: keys[p.envKey], priority: p.priority,
  }));
  if (providers.length === 0) {
    const tiers: ComplexityTier[] = ["simple", "medium", "complex", "expert"];
    const startIdx = tiers.indexOf(tier);
    for (let i = startIdx + 1; i < tiers.length; i++) {
      const fallback = PROVIDER_REGISTRY.filter((p) => p.tier === tiers[i] && keys[p.envKey]);
      if (fallback.length > 0) {
        providers = fallback.sort((a, b) => a.priority - b.priority).map((p) => ({
          id: p.id, tier: p.tier, model: p.model, baseUrl: p.baseUrl,
          apiKey: keys[p.envKey], priority: p.priority,
        }));
        actualTier = tiers[i];
        break;
      }
    }
  }

  // If still nothing, try downward (simpler) tiers
  if (providers.length === 0) {
    const tiers: ComplexityTier[] = ["simple", "medium", "complex", "expert"];
    const startIdx = tiers.indexOf(tier);
    for (let i = startIdx - 1; i >= 0; i--) {
      const fallback = PROVIDER_REGISTRY.filter((p) => p.tier === tiers[i] && keys[p.envKey]);
      if (fallback.length > 0) {
        providers = fallback.sort((a, b) => a.priority - b.priority).map((p) => ({
          id: p.id, tier: p.tier, model: p.model, baseUrl: p.baseUrl,
          apiKey: keys[p.envKey], priority: p.priority,
        }));
        actualTier = tiers[i];
        break;
      }
    }
  }

  const reason = providers.length > 0
    ? `Routed to ${actualTier} tier: ${providers.map((p) => `${p.id}/${p.model}`).join(" → ")}`
    : "No configured providers found for any tier";

  return { tier: actualTier, providers, reason };
}

/**
 * Get all API keys that are set (without revealing their values).
 */
export function getConfiguredProviders(): string[] {
  const keys = discoverAvailableKeys();
  const result: string[] = [];
  for (const p of PROVIDER_REGISTRY) {
    if (keys[p.envKey] && !result.includes(p.id)) {
      result.push(p.id);
    }
  }
  return result;
}

// Re-export the re-built provider list for agent-loop.ts to use
export { PROVIDER_REGISTRY };
