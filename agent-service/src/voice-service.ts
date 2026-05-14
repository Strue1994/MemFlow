/**
 * T3.2: Real-time Voice System
 * TTS (Text-to-Speech) + Speech Recognition + Google Meet/Twilio integration
 */

export interface VoiceConfig {
  ttsEngine: "openai" | "elevenlabs" | "edge";
  sttEngine: "whisper" | "google";
  apiKey?: string;
}

export class VoiceService {
  private config: VoiceConfig;

  constructor(config?: Partial<VoiceConfig>) {
    this.config = {
      ttsEngine: "edge",
      sttEngine: "whisper",
      apiKey: process.env.OPENAI_API_KEY,
      ...config,
    };
  }

  async synthesize(text: string, voice = "alloy"): Promise<ArrayBuffer> {
    if (this.config.ttsEngine === "openai") {
      const resp = await fetch("https://api.openai.com/v1/audio/speech", {
        method: "POST",
        headers: {
          "Authorization": `Bearer ${this.config.apiKey}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ model: "tts-1", input: text, voice, response_format: "mp3" }),
      });
      if (!resp.ok) throw new Error(`TTS failed: ${resp.status}`);
      return resp.arrayBuffer();
    }
    throw new Error(`TTS engine ${this.config.ttsEngine} not implemented`);
  }

  async transcribe(audio: Blob): Promise<string> {
    if (this.config.sttEngine === "whisper") {
      const form = new FormData();
      form.append("file", audio, "audio.webm");
      form.append("model", "whisper-1");
      const resp = await fetch("https://api.openai.com/v1/audio/transcriptions", {
        method: "POST",
        headers: { "Authorization": `Bearer ${this.config.apiKey}` },
        body: form,
      });
      if (!resp.ok) throw new Error(`STT failed: ${resp.status}`);
      const data = await resp.json();
      return data.text || "";
    }
    throw new Error(`STT engine ${this.config.sttEngine} not implemented`);
  }

  /** Generate Twilio-compatible TwiML for voice calls */
  generateTwilioTwiML(message: string): string {
    return `<?xml version="1.0" encoding="UTF-8"?>
<Response>
  <Say voice="alice" language="en-US">${this.escapeXml(message)}</Say>
</Response>`;
  }

  private escapeXml(s: string): string {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  }
}
