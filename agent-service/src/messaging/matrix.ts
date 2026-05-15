/**
 * Matrix Adapter — Matrix protocol via homeserver API
 * Config: { homeserverUrl, accessToken, userId }
 */
import { registerAdapter, type ParsedMessage, type MessageAdapter } from "./adapter";

class MatrixAdapter implements MessageAdapter {
  platform = "matrix";
  parse(raw: any): ParsedMessage {
    return { userId: raw.sender || raw.user_id || "unknown", text: raw.content?.body || raw.body || "", raw, platform: "matrix" };
  }
  async send(userId: string, reply: string): Promise<void> {
    const cfg = this.getConfig();
    if (!cfg?.homeserverUrl || !cfg?.accessToken) throw new Error("Matrix not configured");
    const hs = cfg.homeserverUrl.replace(/\/+$/, "");
    const resp = await fetch(`${hs}/_matrix/client/v3/rooms/${encodeURIComponent(userId)}/send/m.room.message`, {
      method: "PUT",
      headers: { Authorization: `Bearer ${cfg.accessToken}`, "Content-Type": "application/json" },
      body: JSON.stringify({ msgtype: "m.text", body: reply }),
    });
    if (!resp.ok) throw new Error(`Matrix error: ${resp.status}`);
  }
  private getConfig(): Record<string, string> | null {
    try { const { getChannels } = require("../provider-config"); return getChannels().find((c: any) => c.id === "matrix" && c.enabled)?.config || null; } catch { return null; }
  }
}
registerAdapter("matrix", new MatrixAdapter());
