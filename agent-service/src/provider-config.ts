/**
 * Provider Config — persistent multi-provider configuration
 *
 * Replaces env-var-only approach. Users can add/remove/edit providers
 * via API, persisted to .memflow-runtime/config/providers.json
 *
 * On startup, env vars are imported into the file config if no file exists yet.
 */

import { promises as fs } from "node:fs";
import path from "node:path";
import * as crypto from "node:crypto";
import { type ComplexityTier } from "./types";

// ---- Types ----

export interface ProviderEntry {
  id: string;            // "openai", "groq", "gemini", etc.
  label: string;         // display name
  enabled: boolean;
  apiKey: string;
  baseUrl: string;
  model: string;
  tier: ComplexityTier;  // routing tier hint
  priority: number;      // lower = preferred within tier
}

export interface ChannelEntry {
  id: string;            // "telegram", "discord", "wechat", etc.
  label: string;
  enabled: boolean;
  config: Record<string, string>;  // platform-specific (bot token, webhook secret, etc.)
}

export interface SetupStatus {
  /** True if at least one provider is configured and enabled */
  hasProviders: boolean;
  /** True if at least one channel is configured */
  hasChannels: boolean;
  /** Total configured providers */
  providerCount: number;
  /** Total enabled providers */
  enabledProviders: number;
  /** List of configured channel ids */
  channels: string[];
  /** List of enabled provider ids */
  providers: string[];
  /** Whether setup wizard should show */
  needsSetup: boolean;
}

interface ConfigFile {
  version: number;
  providers: ProviderEntry[];
  channels: ChannelEntry[];
  updatedAt: string;
}

// ---- Default provider templates (pre-filled, user just adds API key) ----

const PROVIDER_TEMPLATES: Omit<ProviderEntry, "apiKey" | "enabled">[] = [
  // Tier 1 — Cheap/Fast
  { id: "groq", label: "Groq (Fast)", baseUrl: "https://api.groq.com/openai/v1", model: "llama-3.3-70b-versatile", tier: "simple", priority: 1 },
  { id: "deepseek", label: "DeepSeek", baseUrl: "https://api.deepseek.com/v1", model: "deepseek-chat", tier: "simple", priority: 2 },
  { id: "gemini-flash", label: "Gemini Flash", baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai", model: "gemini-2.0-flash", tier: "simple", priority: 3 },
  { id: "perplexity", label: "Perplexity", baseUrl: "https://api.perplexity.ai", model: "sonar-pro", tier: "simple", priority: 4 },
  { id: "xai", label: "xAI Grok", baseUrl: "https://api.x.ai/v1", model: "grok-2-latest", tier: "simple", priority: 5 },

  // Tier 2 — Balanced
  { id: "openai", label: "OpenAI", baseUrl: "https://api.openai.com/v1", model: "gpt-4o-mini", tier: "medium", priority: 1 },
  { id: "anthropic", label: "Anthropic", baseUrl: "https://api.anthropic.com/v1", model: "claude-3.5-haiku", tier: "medium", priority: 2 },
  { id: "together", label: "Together AI", baseUrl: "https://api.together.xyz/v1", model: "meta-llama/Llama-3.3-70B-Instruct-Turbo", tier: "medium", priority: 3 },
  { id: "mistral", label: "Mistral", baseUrl: "https://api.mistral.ai/v1", model: "mistral-large-latest", tier: "medium", priority: 4 },
  { id: "ollama", label: "Ollama (Local)", baseUrl: "http://localhost:11434/v1", model: "llama3.2", tier: "medium", priority: 5 },

  // Tier 3 — Powerful
  { id: "openai-powerful", label: "OpenAI GPT-4o", baseUrl: "https://api.openai.com/v1", model: "gpt-4o", tier: "complex", priority: 1 },
  { id: "anthropic-powerful", label: "Anthropic Claude Sonnet", baseUrl: "https://api.anthropic.com/v1", model: "claude-sonnet-4-20250514", tier: "complex", priority: 2 },
  { id: "gemini-pro", label: "Gemini Pro", baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai", model: "gemini-2.0-pro", tier: "complex", priority: 3 },

  // Tier 4 — Expert
  { id: "openai-expert", label: "OpenAI o3-mini", baseUrl: "https://api.openai.com/v1", model: "o3-mini", tier: "expert", priority: 1 },
  { id: "anthropic-expert", label: "Anthropic Claude Opus", baseUrl: "https://api.anthropic.com/v1", model: "claude-opus-4-20250514", tier: "expert", priority: 2 },
  { id: "deepseek-expert", label: "DeepSeek Reasoner", baseUrl: "https://api.deepseek.com/v1", model: "deepseek-reasoner", tier: "expert", priority: 3 },
];

const CHANNEL_TEMPLATES: Omit<ChannelEntry, "enabled" | "config">[] = [
  { id: "telegram", label: "Telegram" },
  { id: "discord", label: "Discord" },
  { id: "slack", label: "Slack" },
  { id: "whatsapp", label: "WhatsApp" },
  { id: "wechat", label: "WeChat" },
  { id: "feishu", label: "Feishu / Lark" },
  { id: "signal", label: "Signal" },
  { id: "teams", label: "Microsoft Teams" },
  { id: "google-chat", label: "Google Chat" },
  { id: "line", label: "LINE" },
  { id: "email", label: "Email (SMTP)" },
  { id: "matrix", label: "Matrix" },
  { id: "imessage", label: "iMessage (macOS)" },
];

// ---- Config file path ----

function getConfigDir(): string {
  const root = process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(__dirname, "..", "..", ".memflow-runtime");
  return path.resolve(root, "config");
}

function getConfigPath(): string {
  return path.resolve(getConfigDir(), "providers.json");
}

// ---- Read / Write ----

let cachedConfig: ConfigFile | null = null;

function getDefaultConfig(): ConfigFile {
  return {
    version: 1,
    providers: [],
    channels: [],
    updatedAt: new Date().toISOString(),
  };
}

async function readConfig(): Promise<ConfigFile> {
  if (cachedConfig) return cachedConfig;

  try {
    const raw = await fs.readFile(getConfigPath(), "utf8");
    cachedConfig = decryptConfig(JSON.parse(raw) as ConfigFile);
    return cachedConfig;
  } catch {
    // First run: import from env vars
    const defaults = getDefaultConfig();
    const envImports = importFromEnv();
    defaults.providers = envImports.providers;
    defaults.channels = envImports.channels;
    cachedConfig = defaults;
    // Silently save on first read
    try {
      await fs.mkdir(getConfigDir(), { recursive: true });
      await fs.writeFile(getConfigPath(), JSON.stringify(defaults, null, 2), "utf8");
    } catch { /* best-effort */ }
    return defaults;
  }
}

async function writeConfig(cfg: ConfigFile): Promise<void> {
  cfg.updatedAt = new Date().toISOString();
  cachedConfig = cfg;
  await fs.mkdir(getConfigDir(), { recursive: true });
  await fs.writeFile(getConfigPath(), JSON.stringify(encryptConfig(cfg), null, 2), "utf8");
}

function getEncKey(): Buffer | null {
  const raw = process.env.ENCRYPTION_KEY?.trim();
  if (!raw) return null;
  // derive 32-byte key
  return crypto.createHash("sha256").update(raw).digest();
}

function enc(value: string): string {
  const key = getEncKey();
  if (!key) return value;
  const iv = crypto.randomBytes(12);
  const cipher = crypto.createCipheriv("aes-256-gcm", key, iv);
  const ct = Buffer.concat([cipher.update(value, "utf8"), cipher.final()]);
  const tag = cipher.getAuthTag();
  return `enc:v1:${iv.toString("base64")}:${tag.toString("base64")}:${ct.toString("base64")}`;
}

function dec(value: string): string {
  const key = getEncKey();
  if (!key) return value;
  if (!value.startsWith("enc:v1:")) return value;
  const [, , ivB64, tagB64, ctB64] = value.split(":");
  const decipher = crypto.createDecipheriv("aes-256-gcm", key, Buffer.from(ivB64, "base64"));
  decipher.setAuthTag(Buffer.from(tagB64, "base64"));
  const pt = Buffer.concat([decipher.update(Buffer.from(ctB64, "base64")), decipher.final()]);
  return pt.toString("utf8");
}

function encryptConfig(cfg: ConfigFile): ConfigFile {
  return {
    ...cfg,
    providers: cfg.providers.map((p) => ({ ...p, apiKey: p.apiKey ? enc(p.apiKey) : p.apiKey })),
    channels: cfg.channels.map((c) => ({
      ...c,
      config: Object.fromEntries(Object.entries(c.config || {}).map(([k, v]) => {
        const sensitive = /token|secret|key|password|passwd/i.test(k);
        return [k, sensitive && typeof v === "string" ? enc(v) : v];
      })),
    })),
  };
}

function decryptConfig(cfg: ConfigFile): ConfigFile {
  return {
    ...cfg,
    providers: cfg.providers.map((p) => ({ ...p, apiKey: p.apiKey ? dec(p.apiKey) : p.apiKey })),
    channels: cfg.channels.map((c) => ({
      ...c,
      config: Object.fromEntries(Object.entries(c.config || {}).map(([k, v]) => {
        const val = typeof v === "string" ? dec(v) : v;
        return [k, val as string];
      })),
    })),
  };
}

// ---- Import from env vars (first-run migration) ----

const ENV_KEY_MAP: Record<string, { id: string; template: string }> = {
  OPENAI_API_KEY: { id: "openai", template: "openai" },
  ANTHROPIC_API_KEY: { id: "anthropic", template: "anthropic" },
  GROQ_API_KEY: { id: "groq", template: "groq" },
  DEEPSEEK_API_KEY: { id: "deepseek", template: "deepseek" },
  GEMINI_API_KEY: { id: "gemini-flash", template: "gemini-flash" },
  TOGETHER_API_KEY: { id: "together", template: "together" },
  PERPLEXITY_API_KEY: { id: "perplexity", template: "perplexity" },
  XAI_API_KEY: { id: "xai", template: "xai" },
  MISTRAL_API_KEY: { id: "mistral", template: "mistral" },
};

function importFromEnv(): { providers: ProviderEntry[]; channels: ChannelEntry[] } {
  const providers: ProviderEntry[] = [];
  const seen = new Set<string>();

  for (const [envVar, mapping] of Object.entries(ENV_KEY_MAP)) {
    const val = process.env[envVar]?.trim();
    if (!val) continue;
    if (seen.has(mapping.id)) continue;
    seen.add(mapping.id);

    const tmpl = PROVIDER_TEMPLATES.find((t) => t.id === mapping.template);
    if (tmpl) {
      providers.push({
        ...tmpl,
        apiKey: val,
        enabled: true,
      });
    }
  }

  // Also check LLM settings for legacy support
  if (!seen.has("openai") && process.env.OPENAI_API_KEY?.trim()) {
    // Already handled above in ENV_KEY_MAP
  }

  return { providers, channels: [] };
}

// ---- Public API ----

export async function getProviders(): Promise<ProviderEntry[]> {
  const cfg = await readConfig();
  return cfg.providers;
}

export async function setProviders(providers: ProviderEntry[]): Promise<ProviderEntry[]> {
  const cfg = await readConfig();
  cfg.providers = providers;
  await writeConfig(cfg);
  return providers;
}

export async function addOrUpdateProvider(
  id: string,
  updates: Partial<ProviderEntry>,
): Promise<ProviderEntry> {
  const cfg = await readConfig();
  const idx = cfg.providers.findIndex((p) => p.id === id);
  if (idx >= 0) {
    cfg.providers[idx] = { ...cfg.providers[idx], ...updates, id };
  } else {
    const tmpl = PROVIDER_TEMPLATES.find((t) => t.id === id);
    cfg.providers.push({
      ...tmpl!,
      id,
      apiKey: updates.apiKey || "",
      enabled: updates.enabled ?? true,
      ...updates,
    });
  }
  await writeConfig(cfg);
  return cfg.providers.find((p) => p.id === id)!;
}

export async function removeProvider(id: string): Promise<boolean> {
  const cfg = await readConfig();
  const len = cfg.providers.length;
  cfg.providers = cfg.providers.filter((p) => p.id !== id);
  if (cfg.providers.length !== len) {
    await writeConfig(cfg);
    return true;
  }
  return false;
}

export async function getEnabledProviders(): Promise<ProviderEntry[]> {
  const cfg = await readConfig();
  return cfg.providers.filter((p) => p.enabled && p.apiKey);
}

export async function getChannels(): Promise<ChannelEntry[]> {
  const cfg = await readConfig();
  return cfg.channels;
}

export async function addOrUpdateChannel(
  id: string,
  updates: Partial<ChannelEntry>,
): Promise<ChannelEntry> {
  const cfg = await readConfig();
  const idx = cfg.channels.findIndex((c) => c.id === id);
  const tmpl = CHANNEL_TEMPLATES.find((t) => t.id === id);
  if (idx >= 0) {
    cfg.channels[idx] = { ...cfg.channels[idx], ...updates, id };
  } else {
    cfg.channels.push({
      ...tmpl!,
      id,
      label: updates.label || tmpl?.label || id,
      enabled: updates.enabled ?? false,
      config: updates.config || {},
    });
  }
  await writeConfig(cfg);
  return cfg.channels.find((c) => c.id === id)!;
}

export async function removeChannel(id: string): Promise<boolean> {
  const cfg = await readConfig();
  const len = cfg.channels.length;
  cfg.channels = cfg.channels.filter((c) => c.id !== id);
  if (cfg.channels.length !== len) {
    await writeConfig(cfg);
    return true;
  }
  return false;
}

export async function getSetupStatus(): Promise<SetupStatus> {
  const cfg = await readConfig();
  const enabledProviders = cfg.providers.filter((p) => p.enabled && p.apiKey);
  return {
    hasProviders: enabledProviders.length > 0,
    hasChannels: cfg.channels.some((c) => c.enabled),
    providerCount: cfg.providers.length,
    enabledProviders: enabledProviders.length,
    channels: cfg.channels.filter((c) => c.enabled).map((c) => c.id),
    providers: enabledProviders.map((p) => p.id),
    needsSetup: enabledProviders.length === 0,
  };
}

export async function resetConfig(): Promise<void> {
  cachedConfig = null;
  const defaults = getDefaultConfig();
  await writeConfig(defaults);
}

/** Get provider templates (for the setup wizard dropdown) */
export function getProviderTemplates() {
  return PROVIDER_TEMPLATES.map((t) => ({
    id: t.id,
    label: t.label,
    baseUrl: t.baseUrl,
    model: t.model,
    tier: t.tier,
  }));
}

export function getChannelTemplates() {
  return CHANNEL_TEMPLATES.map((t) => ({
    id: t.id,
    label: t.label,
  }));
}
