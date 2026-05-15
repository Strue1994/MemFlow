/**
 * Signal Adapter — sends/receives messages via Signal Messenger REST API
 *
 * Requires a Signal messenger bridge (signal-cli-rest-api or similar)
 * Config via channel config:
 *   { "signalApiUrl": "http://localhost:8080", "phoneNumber": "+1234567890" }
 */

import { registerAdapter, type ParsedMessage, type MessageAdapter } from "./adapter";

class SignalAdapter implements MessageAdapter {
  platform = "signal";

  parse(raw: any): ParsedMessage {
    const userId = raw.envelope?.sourceUuid || raw.sourceNumber || raw.source || "unknown";
    const text = raw.envelope?.dataMessage?.message || raw.dataMessage?.message || "";
    return { userId, text, raw, platform: "signal" };
  }

  async send(userId: string, reply: string): Promise<void> {
    const config = this.getChannelConfig();
    if (!config?.signalApiUrl) throw new Error("Signal API URL not configured");

    const resp = await fetch(`${config.signalApiUrl}/v2/send`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        number: config.phoneNumber || "+00000000000",
        recipients: [userId],
        message: reply,
      }),
    });

    if (!resp.ok) {
      const err = await resp.text();
      throw new Error(`Signal send error: ${resp.status} ${err.slice(0, 200)}`);
    }
  }

  private getChannelConfig(): Record<string, string> | null {
    try {
      const { getChannels } = require("../provider-config");
      const channels = getChannels() as any[];
      return channels.find((c: any) => c.id === "signal" && c.enabled)?.config || null;
    } catch { return null; }
  }
}

registerAdapter("signal", new SignalAdapter());
export default SignalAdapter;
