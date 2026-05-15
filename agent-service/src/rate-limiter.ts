import { Request, Response, NextFunction } from "express";

type Bucket = {
  tokens: number;
  lastRefillMs: number;
};

const buckets = new Map<string, Bucket>();

const RPM = Number(process.env.RATE_LIMIT_RPM || 100);
const CAPACITY = Math.max(1, RPM);
const REFILL_PER_MS = CAPACITY / 60000;

function getClientIp(req: Request): string {
  return (
    (req.headers["x-forwarded-for"] as string)?.split(",")[0]?.trim() ||
    req.socket.remoteAddress ||
    "unknown"
  );
}

export function rateLimiter(req: Request, res: Response, next: NextFunction): void {
  const ip = getClientIp(req);
  const now = Date.now();
  const existing = buckets.get(ip) || { tokens: CAPACITY, lastRefillMs: now };

  const elapsed = Math.max(0, now - existing.lastRefillMs);
  existing.tokens = Math.min(CAPACITY, existing.tokens + elapsed * REFILL_PER_MS);
  existing.lastRefillMs = now;

  if (existing.tokens < 1) {
    res.setHeader("Retry-After", "60");
    res.status(429).json({ error: "rate limit exceeded" });
    buckets.set(ip, existing);
    return;
  }

  existing.tokens -= 1;
  buckets.set(ip, existing);
  next();
}
