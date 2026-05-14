export type LLMProvider =
  | "deepai"
  | "gmn"
  | "openai"
  | "openrouter"
  | "anthropic"
  | "google"
  | "deepseek"
  | "groq"
  | "mistral"
  | "xai"
  | "ollama"
  | "openai-compatible";

export type LLMApiStyle = "openai" | "anthropic" | "google";

export interface LLMProviderPreset {
  id: LLMProvider;
  label: string;
  labelZh: string;
  apiStyle: LLMApiStyle;
  defaultBaseUrl: string;
  defaultModel: string;
  modelSuggestions: string[];
  note: string;
}

export const LLM_PROVIDER_PRESETS: LLMProviderPreset[] = [
  {
    id: "deepai",
    label: "DeepAI",
    labelZh: "DeepAI",
    apiStyle: "openai",
    defaultBaseUrl: "https://ai.nexapi.cn",
    defaultModel: "gpt-5.4",
    modelSuggestions: ["gpt-5.3-codex", "gpt-5.4", "gpt-5.4-mini"],
    note: "OpenAI-compatible DeepAI route with the fixed local model set.",
  },
  {
    id: "gmn",
    label: "GMN",
    labelZh: "GMN",
    apiStyle: "openai",
    defaultBaseUrl: "https://gmncode.cn/v1",
    defaultModel: "gpt-5.4",
    modelSuggestions: ["gpt-5.4"],
    note: "Matches the OpenClaw team default route.",
  },
  {
    id: "openai",
    label: "OpenAI",
    labelZh: "OpenAI",
    apiStyle: "openai",
    defaultBaseUrl: "https://api.openai.com/v1",
    defaultModel: "gpt-4o-mini",
    modelSuggestions: ["gpt-4o-mini", "gpt-4o", "o3-mini"],
    note: "Direct OpenAI chat completions.",
  },
  {
    id: "openrouter",
    label: "OpenRouter",
    labelZh: "OpenRouter",
    apiStyle: "openai",
    defaultBaseUrl: "https://openrouter.ai/api/v1",
    defaultModel: "openrouter/auto",
    modelSuggestions: [
      "openrouter/auto",
      "anthropic/claude-sonnet-4",
      "google/gemini-2.5-pro",
      "deepseek/deepseek-v3.2",
      "x-ai/grok-4-fast",
    ],
    note: "OpenClaw-style broad provider gateway with many model families.",
  },
  {
    id: "anthropic",
    label: "Anthropic",
    labelZh: "Anthropic",
    apiStyle: "anthropic",
    defaultBaseUrl: "https://api.anthropic.com/v1",
    defaultModel: "claude-sonnet-4-20250514",
    modelSuggestions: [
      "claude-sonnet-4-20250514",
      "claude-opus-4-20250514",
      "claude-3-5-haiku-20241022",
    ],
    note: "Direct Anthropic messages API.",
  },
  {
    id: "google",
    label: "Google",
    labelZh: "Google / Gemini",
    apiStyle: "google",
    defaultBaseUrl: "https://generativelanguage.googleapis.com/v1beta",
    defaultModel: "gemini-2.5-flash",
    modelSuggestions: ["gemini-2.5-flash", "gemini-2.5-pro", "gemini-2.0-flash"],
    note: "Direct Gemini generateContent API.",
  },
  {
    id: "deepseek",
    label: "DeepSeek",
    labelZh: "DeepSeek",
    apiStyle: "openai",
    defaultBaseUrl: "https://api.deepseek.com/v1",
    defaultModel: "deepseek-chat",
    modelSuggestions: ["deepseek-chat", "deepseek-reasoner"],
    note: "OpenAI-compatible DeepSeek API.",
  },
  {
    id: "groq",
    label: "Groq",
    labelZh: "Groq",
    apiStyle: "openai",
    defaultBaseUrl: "https://api.groq.com/openai/v1",
    defaultModel: "llama-3.3-70b-versatile",
    modelSuggestions: ["llama-3.3-70b-versatile", "qwen-qwq-32b", "deepseek-r1-distill-llama-70b"],
    note: "OpenAI-compatible low-latency Groq route.",
  },
  {
    id: "mistral",
    label: "Mistral",
    labelZh: "Mistral",
    apiStyle: "openai",
    defaultBaseUrl: "https://api.mistral.ai/v1",
    defaultModel: "mistral-small-latest",
    modelSuggestions: ["mistral-small-latest", "mistral-large-latest", "codestral-latest"],
    note: "OpenAI-compatible Mistral route.",
  },
  {
    id: "xai",
    label: "xAI",
    labelZh: "xAI / Grok",
    apiStyle: "openai",
    defaultBaseUrl: "https://api.x.ai/v1",
    defaultModel: "grok-3-mini",
    modelSuggestions: ["grok-3-mini", "grok-4-fast", "grok-code-fast-1"],
    note: "Direct xAI Grok route.",
  },
  {
    id: "ollama",
    label: "Ollama",
    labelZh: "Ollama",
    apiStyle: "openai",
    defaultBaseUrl: "http://127.0.0.1:11434/v1",
    defaultModel: "qwen2.5-coder:7b",
    modelSuggestions: ["qwen2.5-coder:7b", "llama3.1:8b", "deepseek-r1:latest"],
    note: "Local OpenAI-compatible endpoint.",
  },
  {
    id: "openai-compatible",
    label: "Custom Compatible",
    labelZh: "自定义兼容接口",
    apiStyle: "openai",
    defaultBaseUrl: "",
    defaultModel: "gpt-4o-mini",
    modelSuggestions: ["gpt-4o-mini"],
    note: "Use your own OpenAI-compatible gateway or self-hosted route.",
  },
];

export function getProviderPreset(provider: string): LLMProviderPreset {
  return (
    LLM_PROVIDER_PRESETS.find((item) => item.id === provider) ||
    LLM_PROVIDER_PRESETS.find((item) => item.id === "openai-compatible")!
  );
}
