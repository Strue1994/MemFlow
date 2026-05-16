/**
 * 🔒 Fixed: JWT auth now verifies signature instead of blindly decoding
 */

import { Request, Response, NextFunction } from "express";
import * as crypto from "node:crypto";

export interface AuthConfig {
  jwtSecret?: string;
  apiKeys: string[];
  enabled: boolean;
}

const config: AuthConfig = {
  jwtSecret: process.env.JWT_SECRET,
  apiKeys: (process.env.API_KEYS || "").split(",").filter(Boolean),
  enabled: process.env.AUTH_ENABLED !== "false",
};

export interface AuthenticatedRequest extends Request {
  userId?: string;
  roles?: string[];
}

function verifyJWT(token: string, secret: string): Record<string, any> | null {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return null;

    // Verify signature using HMAC-SHA256
    const header = parts[0];
    const payload = parts[1];
    const signature = parts[2];

    const expectedSig = crypto
      .createHmac("sha256", secret)
      .update(`${header}.${payload}`)
      .digest("base64url");

    // Constant-time comparison to prevent timing attacks
    if (signature.length !== expectedSig.length) return null;
    let match = 0;
    for (let i = 0; i < signature.length; i++) {
      match |= signature.charCodeAt(i) ^ expectedSig.charCodeAt(i);
    }
    if (match !== 0) return null;

    // Decode payload
    const decoded = JSON.parse(Buffer.from(payload, "base64url").toString());
    return decoded;
  } catch {
    return null;
  }
}

export function authMiddleware(req: AuthenticatedRequest, res: Response, next: NextFunction): void {
  if (!config.enabled) { next(); return; }

  // Web UI static files should load without auth
  if (req.path === "/" || req.path.startsWith("/assets/")) { next(); return; }

  const publicPaths = new Set([
    "/health",
    "/ready",
    "/live",
    "/setup/status",
  ]);
  if (publicPaths.has(req.path)) { next(); return; }

  const authHeader = req.headers.authorization;
  if (!authHeader) {
    res.status(401).json({ error: "Missing Authorization header" });
    return;
  }

  if (authHeader.startsWith("Bearer ")) {
    const token = authHeader.slice(7);

    // API key check
    if (config.apiKeys.includes(token)) {
      req.userId = "api-user";
      req.roles = ["admin"];
      next(); return;
    }

    // JWT verification (signature-verified, not just base64-decoded)
    if (config.jwtSecret) {
      const payload = verifyJWT(token, config.jwtSecret);
      if (payload) {
        req.userId = payload.sub || payload.userId;
        req.roles = payload.roles || ["user"];
        next(); return;
      }
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

