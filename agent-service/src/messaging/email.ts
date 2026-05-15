/**
 * Email Adapter — SMTP for email messaging (no external dependencies)
 * Config: { smtpHost, smtpPort, smtpUser, smtpPass, emailAddress }
 */
import { registerAdapter, type ParsedMessage, type MessageAdapter } from "./adapter";
import * as net from "node:net";
import * as tls from "node:tls";

class EmailAdapter implements MessageAdapter {
  platform = "email";
  parse(raw: any): ParsedMessage {
    return { userId: raw.from || raw.sender || "unknown", text: raw.text || raw.body || "", raw, platform: "email" };
  }
  async send(userId: string, reply: string): Promise<void> {
    const cfg = this.getConfig();
    if (!cfg?.smtpHost) throw new Error("Email SMTP not configured");
    const port = parseInt(cfg.smtpPort || "587");
    const isSecure = cfg.smtpSecure === "true";
    const sock = isSecure
      ? tls.connect(port, cfg.smtpHost, { rejectUnauthorized: false })
      : net.connect(port, cfg.smtpHost);
    const user = cfg.smtpUser || "";
    const pass = cfg.smtpPass || "";
    const from = cfg.emailAddress || user;
    const domain = cfg.smtpHost;

    await new Promise<void>((resolve, reject) => {
      let step = 0;
      let buffer = "";
      const send = (cmd: string) => { sock.write(cmd + "\r\n"); };
      sock.setTimeout(10000);
      sock.on("data", async (data) => {
        buffer += data.toString();
        if (!buffer.includes("\r\n")) return;
        const lines = buffer.split("\r\n");
        buffer = "";
        const code = parseInt(lines[0]?.substring(0, 3) || "500");
        if (code >= 400) { sock.destroy(); reject(new Error(`SMTP error: ${lines[0]}`)); return; }
        step++;
        try {
          if (step === 1) { send(`EHLO ${domain}`); }
          else if (step === 2 && lines[0]?.includes("STARTTLS")) { send("STARTTLS"); }
          else if (step === 3 && lines[0]?.includes("250")) { /* upgraded */ send(`AUTH LOGIN`); }
          else if (step === 4) { send(Buffer.from(user).toString("base64")); }
          else if (step === 5) { send(Buffer.from(pass).toString("base64")); }
          else if (step === 6) { send(`MAIL FROM:<${from}>`); }
          else if (step === 7) { send(`RCPT TO:<${userId}>`); }
          else if (step === 8) { send("DATA"); }
          else if (step === 9) {
            send(`From: ${from}\r\nTo: ${userId}\r\nSubject: MemFlow Response\r\n\r\n${reply}\r\n.`);
          } else if (step === 10) { send("QUIT"); resolve(); }
        } catch (e: any) { sock.destroy(); reject(e); }
      });
      sock.on("error", reject);
      sock.on("timeout", () => { sock.destroy(); reject(new Error("SMTP timeout")); });
    });
  }
  private getConfig(): Record<string, string> | null {
    try { const { getChannels } = require("../provider-config"); return getChannels().find((c: any) => c.id === "email" && c.enabled)?.config || null; } catch { return null; }
  }
}
registerAdapter("email", new EmailAdapter());
