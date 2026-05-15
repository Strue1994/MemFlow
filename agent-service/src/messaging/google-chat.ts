import { registerAdapter, type ParsedMessage, type MessageAdapter } from "./adapter";
class GoogleChatAdapter implements MessageAdapter {
  platform = "google-chat";
  parse(raw: any): ParsedMessage { return { userId: raw.user?.name || "unknown", text: raw.text || "", raw, platform: "google-chat" }; }
  async send(_userId: string, reply: string): Promise<void> {
    const config = this.getConfig(); if (!config?.webhookUrl) throw new Error("Google Chat webhook not configured");
    await fetch(config.webhookUrl, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ text: reply }) });
  }
  private getConfig(): Record<string, string> | null { try { const { getChannels } = require("../provider-config"); return getChannels().find((c: any) => c.id === "google-chat" && c.enabled)?.config || null; } catch { return null; } }
}
registerAdapter("google-chat", new GoogleChatAdapter());
