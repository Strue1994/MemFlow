import { getProviderPreset } from "./llm_catalog";
import type { LLMSettings } from "./llm_settings";

interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

interface ChatResult {
  content: string;
  raw: unknown;
}

function normalizeRoot(baseUrl: string, fallback: string): string {
  return (baseUrl || fallback).replace(/\/$/, "");
}

function textFromAnthropicContent(content: Array<{ type: string; text?: string }> | undefined): string {
  return (content || [])
    .filter((part) => part.type === "text")
    .map((part) => part.text || "")
    .join("");
}

function toAnthropicBody(model: string, messages: ChatMessage[]) {
  let system = "";
  const translated = messages
    .filter((message) => {
      if (message.role === "system") {
        system = system ? `${system}\n${message.content}` : message.content;
        return false;
      }
      return true;
    })
    .map((message) => ({
      role: message.role === "assistant" ? "assistant" : "user",
      content: [{ type: "text", text: message.content }],
    }));

  return {
    model,
    max_tokens: 4096,
    system: system || undefined,
    messages: translated,
  };
}

function toGeminiBody(messages: ChatMessage[]) {
  let systemInstruction: { parts: Array<{ text: string }> } | undefined;
  const contents = messages
    .filter((message) => {
      if (message.role === "system") {
        systemInstruction = { parts: [{ text: message.content }] };
        return false;
      }
      return true;
    })
    .map((message) => ({
      role: message.role === "assistant" ? "model" : "user",
      parts: [{ text: message.content }],
    }));

  return {
    contents,
    systemInstruction,
  };
}

async function callOpenAIStyle(settings: LLMSettings, messages: ChatMessage[]): Promise<ChatResult> {
  const root = normalizeRoot(settings.baseUrl, getProviderPreset(settings.provider).defaultBaseUrl);
  const response = await fetch(`${root}/chat/completions`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${settings.apiKey}`,
    },
    body: JSON.stringify({
      model: settings.model,
      messages,
      max_tokens: 4096,
      temperature: 0.2,
    }),
  });

  const raw = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error((raw as any)?.error?.message || `LLM request failed: ${response.status}`);
  }

  return {
    content: (raw as any)?.choices?.[0]?.message?.content || "",
    raw,
  };
}

async function callAnthropic(settings: LLMSettings, messages: ChatMessage[]): Promise<ChatResult> {
  const root = normalizeRoot(settings.baseUrl, getProviderPreset(settings.provider).defaultBaseUrl);
  const response = await fetch(`${root}/messages`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": settings.apiKey,
      "anthropic-version": "2023-06-01",
    },
    body: JSON.stringify(toAnthropicBody(settings.model, messages)),
  });

  const raw = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error((raw as any)?.error?.message || `Anthropic request failed: ${response.status}`);
  }

  return {
    content: textFromAnthropicContent((raw as any)?.content),
    raw,
  };
}

async function callGoogle(settings: LLMSettings, messages: ChatMessage[]): Promise<ChatResult> {
  const root = normalizeRoot(settings.baseUrl, getProviderPreset(settings.provider).defaultBaseUrl);
  const response = await fetch(`${root}/models/${settings.model}:generateContent`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-goog-api-key": settings.apiKey,
    },
    body: JSON.stringify(toGeminiBody(messages)),
  });

  const raw = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error((raw as any)?.error?.message || `Google request failed: ${response.status}`);
  }

  const candidate = (raw as any)?.candidates?.[0];
  const content = (candidate?.content?.parts || []).map((part: any) => part.text || "").join("");

  return {
    content,
    raw,
  };
}

export async function createChatCompletion(settings: LLMSettings, messages: ChatMessage[]): Promise<ChatResult> {
  const preset = getProviderPreset(settings.provider);

  if (!settings.apiKey) {
    throw new Error("LLM API key is not configured");
  }

  switch (preset.apiStyle) {
    case "anthropic":
      return callAnthropic(settings, messages);
    case "google":
      return callGoogle(settings, messages);
    case "openai":
    default:
      return callOpenAIStyle(settings, messages);
  }
}
