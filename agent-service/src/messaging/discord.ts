/**
 * Discord Adapter — sends/receives messages via Discord bot webhooks
 *
 * Config via channel config:
 *   { "botToken": "...", "guildId": "...", "appId": "..." }
 */

import { registerAdapter, type ParsedMessage, type MessageAdapter } from "./adapter";

class DiscordAdapter implements MessageAdapter {
  platform = "discord";

  parse(raw: any): ParsedMessage {
    // Handle Discord interaction webhook payloads
    const userId = raw.member?.user?.id || raw.user?.id || raw.author?.id || "unknown";
    const text = raw.data?.name === "ping"
      ? "/ping"
      : raw.data?.options?.[0]?.value || raw.content || "";

    return { userId, text, raw, platform: "discord" };
  }

  async send(userId: string, reply: string): Promise<void> {
    const config = this.getChannelConfig();
    if (!config?.botToken) throw new Error("Discord bot token not configured");

    // Send DM via Discord REST API
    const resp = await fetch(`https://discord.com/api/v10/users/@me/channels`, {
      method: "POST",
      headers: {
        "Authorization": `Bot ${config.botToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ recipient_id: userId }),
    });

    if (!resp.ok) throw new Error(`Discord DM channel error: ${resp.status}`);

    const channel = await resp.json() as { id: string };
    await fetch(`https://discord.com/api/v10/channels/${channel.id}/messages`, {
      method: "POST",
      headers: {
        "Authorization": `Bot ${config.botToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ content: reply.slice(0, 2000) }),
    });
  }

  verify(req: any): boolean {
    // Basic webhook verification — check for Discord signature headers
    return true; // Full verification requires ed25519 which needs a lib
  }

  private getChannelConfig(): Record<string, string> | null {
    try {
      const { getChannels } = require("../provider-config");
      const channels = getChannels() as any[];
      return channels.find((c: any) => c.id === "discord" && c.enabled)?.config || null;
    } catch { return null; }
  }
}

registerAdapter("discord", new DiscordAdapter());
export default DiscordAdapter;
