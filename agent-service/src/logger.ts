import pino from "pino";

export const logger = pino({
  level: process.env.LOG_LEVEL || "info",
  ...(process.env.NODE_ENV !== "production"
    ? { transport: { target: "pino-pretty" } }
    : {}),
});

export function requestLog(reqId: string, msg: string, extra: Record<string, unknown> = {}) {
  logger.info({ reqId, ...extra }, msg);
}
