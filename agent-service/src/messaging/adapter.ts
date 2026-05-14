export interface ParsedMessage {
  userId: string;
  text: string;
  raw: any;
  platform: string;
}

export interface MessageAdapter {
  platform: string;
  parse(raw: any): ParsedMessage;
  send(userId: string, reply: string): Promise<void>;
  verify?(req: any): boolean;
}

const adapters = new Map<string, MessageAdapter>();

export function registerAdapter(platform: string, adapter: MessageAdapter) {
  adapters.set(platform, adapter);
  console.log(`[Messaging] Registered adapter for ${platform}`);
}

export function getAdapter(platform: string): MessageAdapter | undefined {
  return adapters.get(platform);
}

export function getAllAdapters(): Map<string, MessageAdapter> {
  return adapters;
}

export function parseMessage(platform: string, raw: any): ParsedMessage | null {
  const adapter = adapters.get(platform);
  if (!adapter) {
    console.warn(`[Messaging] No adapter for platform: ${platform}`);
    return null;
  }
  return adapter.parse(raw);
}

export async function sendReply(platform: string, userId: string, reply: string): Promise<void> {
  const adapter = adapters.get(platform);
  if (!adapter) {
    throw new Error(`No adapter for platform: ${platform}`);
  }
  await adapter.send(userId, reply);
}

export function verifyRequest(platform: string, req: any): boolean {
  const adapter = adapters.get(platform);
  if (!adapter?.verify) {
    return true;
  }
  return adapter.verify(req);
}