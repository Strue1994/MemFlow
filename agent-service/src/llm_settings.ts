import { promises as fs } from "node:fs";
import path from "node:path";
import { getProviderPreset, type LLMProvider, LLM_PROVIDER_PRESETS } from "./llm_catalog";

export { LLM_PROVIDER_PRESETS };

export interface LLMSettings {
  provider: LLMProvider;
  apiKey: string;
  baseUrl: string;
  model: string;
  updatedAt: string | null;
}

const SETTINGS_PATH = (() => {
  if (process.env.MEMFLOW_LLM_SETTINGS_PATH) {
    return process.env.MEMFLOW_LLM_SETTINGS_PATH;
  }
  if (process.env.MEMFLOW_RUNTIME_ROOT) {
    return path.resolve(process.env.MEMFLOW_RUNTIME_ROOT, "config", "llm-settings.json");
  }
  return path.resolve(__dirname, "..", "..", ".memflow-runtime", "config", "llm-settings.json");
})();

let cachedSettings: LLMSettings | null = null;

function sanitizeProvider(provider?: string | null): LLMProvider {
  return (LLM_PROVIDER_PRESETS.find((item) => item.id === provider)?.id || "openai") as LLMProvider;
}

function sanitizeSettings(input?: Partial<LLMSettings> | null): LLMSettings {
  const provider = sanitizeProvider(input?.provider ?? process.env.LLM_PROVIDER);
  const preset = getProviderPreset(provider);
  const apiKey = (input?.apiKey ?? process.env.OPENAI_API_KEY ?? "").trim();
  const baseUrl = (input?.baseUrl ?? process.env.OPENAI_BASE_URL ?? preset.defaultBaseUrl).trim();
  const model = (input?.model ?? process.env.OPENAI_MODEL ?? preset.defaultModel).trim() || preset.defaultModel;

  return {
    provider,
    apiKey,
    baseUrl,
    model,
    updatedAt: input?.updatedAt || null,
  };
}

async function readSettingsFile(): Promise<LLMSettings | null> {
  try {
    const raw = await fs.readFile(SETTINGS_PATH, "utf8");
    const parsed = JSON.parse(raw) as Partial<LLMSettings>;
    return sanitizeSettings(parsed);
  } catch {
    return null;
  }
}

function applySettingsToEnv(settings: LLMSettings): void {
  process.env.LLM_PROVIDER = settings.provider;
  process.env.OPENAI_API_KEY = settings.apiKey;
  process.env.OPENAI_BASE_URL = settings.baseUrl;
  process.env.OPENAI_MODEL = settings.model;
}

export async function getLLMSettings(): Promise<LLMSettings> {
  if (cachedSettings) {
    return cachedSettings;
  }

  const fileSettings = await readSettingsFile();
  cachedSettings = fileSettings || sanitizeSettings();
  applySettingsToEnv(cachedSettings);
  return cachedSettings;
}

export async function saveLLMSettings(nextSettings: Partial<LLMSettings>): Promise<LLMSettings> {
  const current = await getLLMSettings();
  const merged = sanitizeSettings({
    ...current,
    ...nextSettings,
    updatedAt: new Date().toISOString(),
  });

  await fs.mkdir(path.dirname(SETTINGS_PATH), { recursive: true });
  await fs.writeFile(SETTINGS_PATH, JSON.stringify(merged, null, 2), "utf8");

  cachedSettings = merged;
  applySettingsToEnv(merged);
  return merged;
}
