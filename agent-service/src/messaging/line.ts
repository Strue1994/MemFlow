import { registerAdapter, type ParsedMessage, type MessageAdapter } from "./adapter";
class LINEAdapter implements MessageAdapter {
  platform = "line";
  parse(raw: any): ParsedMessage { return { userId: raw.events?.[0]?.source?.userId || "unknown", text: raw.events?.[0]?.message?.text || "", raw, platform: "line" }; }
  async send(userId: string, reply: string): Promise<void> {
    const config = this.getConfig(); if (!config?.channelAccessToken) throw new Error("LINE token not configured");
    await fetch("https://api.line.me/v2/bot/message/push", { method: "POST", headers: { Authorization: `Bearer ${config.channelAccessToken}`, "Content-Type": "application/json" }, body: JSON.stringify({ to: userId, messages: [{ type: "text", text: reply }] }) });
  }
  private getConfig(): Record<string, string> | null { try { const { getChannels } = require("../provider-config"); return getChannels().find((c: any) => c.id === "line" && c.enabled)?.config || null; } catch { return null; } }
}
registerAdapter("line", new LINEAdapter());
