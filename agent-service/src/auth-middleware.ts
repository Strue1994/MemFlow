/**
 * P1.2: Unified Authentication Middleware
 */

import { Request, Response, NextFunction } from "express";

export interface AuthConfig {
  jwtSecret?: string;
  apiKeys: string[];
  enabled: boolean;
}

const config: AuthConfig = {
  jwtSecret: process.env.JWT_SECRET,
  apiKeys: (process.env.API_KEYS || "").split(",").filter(Boolean),
  enabled: process.env.AUTH_ENABLED === "true",
};

export interface AuthenticatedRequest extends Request {
  userId?: string;
  roles?: string[];
}

export function authMiddleware(req: AuthenticatedRequest, res: Response, next: NextFunction): void {
  if (!config.enabled) { next(); return; }

  const authHeader = req.headers.authorization;
  if (!authHeader) {
    res.status(401).json({ error: "Missing Authorization header" });
    return;
  }

  if (authHeader.startsWith("Bearer ")) {
    const token = authHeader.slice(7);
    if (config.apiKeys.includes(token)) {
      req.userId = "api-user";
      req.roles = ["admin"];
      next(); return;
    }
    if (config.jwtSecret) {
      try {
        const payload = JSON.parse(Buffer.from(token.split(".")[1], "base64").toString());
        req.userId = payload.sub || payload.userId;
        req.roles = payload.roles || ["user"];
        next(); return;
      } catch { /* not a valid JWT */ }
    }
  }

  const apiKey = req.headers["x-api-key"] as string;
  if (apiKey && config.apiKeys.includes(apiKey)) {
    req.userId = "api-user";
    req.roles = ["admin"];
    next(); return;
  }

  res.status(401).json({ error: "Invalid or expired authentication" });
}

export function requireRole(...roles: string[]) {
  return (req: AuthenticatedRequest, res: Response, next: NextFunction): void => {
    if (!config.enabled) { next(); return; }
    if (!req.roles || !roles.some((r) => req.roles!.includes(r))) {
      res.status(403).json({ error: "Insufficient permissions" });
      return;
    }
    next();
  };
}
