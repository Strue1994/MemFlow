export function validateAgentExecuteBody(body: any): string | null {
  if (!body || typeof body !== "object") return "body required";
  if (typeof body.text !== "string") return "text must be string";
  if (body.text.length === 0) return "text cannot be empty";
  if (body.text.length > 10000) return "text too long (max 10000)";
  return null;
}

export function validateChatCompletionsBody(body: any): string | null {
  if (!body || typeof body !== "object") return "body required";
  if (!Array.isArray(body.messages)) return "messages must be an array";
  for (const m of body.messages) {
    if (!m || typeof m !== "object") return "message must be object";
    if (typeof m.role !== "string" || typeof m.content !== "string") {
      return "message.role and message.content must be strings";
    }
  }
  return null;
}

export function validateCheckpointSaveBody(body: any): string | null {
  if (!body || typeof body !== "object") return "body required";
  if (typeof body.sessionId !== "string" || body.sessionId.length === 0) {
    return "sessionId must be a non-empty string";
  }
  if (!Array.isArray(body.messages)) return "messages must be an array";
  return null;
}
