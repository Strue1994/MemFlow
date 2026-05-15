import { registerAdapter, type ParsedMessage, type MessageAdapter } from "./adapter";
class TeamsAdapter implements MessageAdapter {
  platform = "teams";
  parse(raw: any): ParsedMessage { return { userId: raw.from?.id || "unknown", text: raw.text || "", raw, platform: "teams" }; }
  async send(userId: string, reply: string): Promise<void> {
    const config = this.getConfig(); if (!config?.webhookUrl) throw new Error("Teams webhook not configured");
    await fetch(config.webhookUrl, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ text: reply }) });
  }
  private getConfig(): Record<string, string> | null { try { const { getChannels } = require("../provider-config"); return getChannels().find((c: any) => c.id === "teams" && c.enabled)?.config || null; } catch { return null; } }
}
registerAdapter("teams", new TeamsAdapter());
