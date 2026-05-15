import pino from "pino";

export const logger = pino({
  level: process.env.LOG_LEVEL || "info",
});

export function requestLog(reqId: string, msg: string, extra: Record<string, unknown> = {}) {
  logger.info({ reqId, ...extra }, msg);
}
