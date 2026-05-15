import { registerAdapter, type ParsedMessage, type MessageAdapter } from "./adapter";
class SlackAdapter implements MessageAdapter {
  platform = "slack";
  parse(raw: any): ParsedMessage {
    return { userId: raw.event?.user || raw.user_id || "unknown", text: raw.event?.text || raw.text || "", raw, platform: "slack" };
  }
  async send(userId: string, reply: string): Promise<void> {
    const config = this.getConfig();
    if (!config?.token) throw new Error("Slack token not configured");
    const resp = await fetch("https://slack.com/api/chat.postMessage", {
      method: "POST", headers: { Authorization: `Bearer ${config.token}`, "Content-Type": "application/json" },
      body: JSON.stringify({ channel: userId, text: reply }),
    });
    if (!resp.ok) throw new Error(`Slack error: ${resp.status}`);
  }
  private getConfig(): Record<string, string> | null {
    try { const { getChannels } = require("../provider-config"); return getChannels().find((c: any) => c.id === "slack" && c.enabled)?.config || null; } catch { return null; }
  }
}
registerAdapter("slack", new SlackAdapter());
