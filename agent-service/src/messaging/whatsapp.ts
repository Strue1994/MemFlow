import { registerAdapter, type ParsedMessage, type MessageAdapter } from "./adapter";
class WhatsAppAdapter implements MessageAdapter {
  platform = "whatsapp";
  parse(raw: any): ParsedMessage {
    const entry = raw.entry?.[0]; const change = entry?.changes?.[0]; const msg = change?.value?.messages?.[0];
    return { userId: msg?.from || "unknown", text: msg?.text?.body || "", raw, platform: "whatsapp" };
  }
  async send(userId: string, reply: string): Promise<void> {
    const config = this.getConfig();
    if (!config?.token || !config?.phoneNumberId) throw new Error("WhatsApp token or phoneNumberId not configured");
    const resp = await fetch(`https://graph.facebook.com/v21.0/${config.phoneNumberId}/messages`, {
      method: "POST", headers: { Authorization: `Bearer ${config.token}`, "Content-Type": "application/json" },
      body: JSON.stringify({ messaging_product: "whatsapp", to: userId, text: { body: reply } }),
    });
    if (!resp.ok) throw new Error(`WhatsApp error: ${resp.status}`);
  }
  private getConfig(): Record<string, string> | null {
    try { const { getChannels } = require("../provider-config"); return getChannels().find((c: any) => c.id === "whatsapp" && c.enabled)?.config || null; } catch { return null; }
  }
}
registerAdapter("whatsapp", new WhatsAppAdapter());
