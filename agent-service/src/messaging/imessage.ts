/**
 * iMessage Adapter — macOS iMessage bridge via AppleScript/Shortcuts
 * Requires: macOS with Messages.app running
 * Config: { bridgeScript: "/path/to/imessage-bridge.sh" }
 *
 * The bridge script should accept: send <phone-or-email> <message>
 */
import { registerAdapter, type ParsedMessage, type MessageAdapter } from "./adapter";
import { execSync } from "node:child_process";

class IMessageAdapter implements MessageAdapter {
  platform = "imessage";
  parse(raw: any): ParsedMessage {
    return { userId: raw.from || raw.sender || "unknown", text: raw.text || "", raw, platform: "imessage" };
  }
  async send(userId: string, reply: string): Promise<void> {
    if (process.platform !== "darwin") {
      console.warn("[iMessage] Not on macOS, cannot send iMessage");
      return;
    }
    const cfg = this.getConfig();
    const script = cfg?.bridgeScript || `/usr/bin/osascript -e 'tell application "Messages" to send "${reply.replace(/"/g, '\\"')}" to buddy "${userId}"'`;
    try {
      execSync(script, { timeout: 15000 });
    } catch (err: any) {
      throw new Error(`iMessage send error: ${err.message}`);
    }
  }
  private getConfig(): Record<string, string> | null {
    try { const { getChannels } = require("../provider-config"); return getChannels().find((c: any) => c.id === "imessage" && c.enabled)?.config || null; } catch { return null; }
  }
}
registerAdapter("imessage", new IMessageAdapter());
