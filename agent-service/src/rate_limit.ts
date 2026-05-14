import rateLimit from 'express-rate-limit';

export const apiRateLimiter = rateLimit({
  windowMs: 60 * 1000,
  max: 60,
  message: { error: 'Too many requests, please try again later' },
  standardHeaders: true,
  legacyHeaders: false,
});

export const sensitiveEndpointLimiter = rateLimit({
  windowMs: 60 * 1000,
  max: 20,
  message: { error: 'Rate limit exceeded for sensitive endpoint' },
});

export function createRateLimiter(maxRequests: number, windowMinutes: number = 1) {
  return rateLimit({
    windowMs: windowMinutes * 60 * 1000,
    max: maxRequests,
    message: { error: 'Rate limit exceeded' },
  });
}

export interface RateLimitConfig {
  windowMs: number;
  max: number;
  skipSuccessfulRequests: boolean;
}

export const DEFAULT_CONFIGS: Record<string, RateLimitConfig> = {
  default: { windowMs: 60000, max: 60, skipSuccessfulRequests: false },
  execute: { windowMs: 60000, max: 20, skipSuccessfulRequests: false },
  create: { windowMs: 60000, max: 10, skipSuccessfulRequests: false },
};

export function getRateLimitConfig(endpoint: string): RateLimitConfig {
  return DEFAULT_CONFIGS[endpoint] || DEFAULT_CONFIGS.default;
}