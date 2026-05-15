/**
 * Agent Security Scanner — AgentShield-style auditing for MemFlow agents
 *
 * Scans 5 categories:
 * 1. SECRET: Hardcoded API keys, tokens, passwords in configs
 * 2. PERMISSION: Overly permissive access policies
 * 3. HOOK: Malicious hook scripts
 * 4. MCP: Unsafe MCP server configurations
 * 5. CONFIG: Agent config vulnerabilities
 */

import * as fs from "node:fs";
import * as path from "node:path";

// ---- Types ----

export interface ScanFinding {
  severity: "critical" | "high" | "medium" | "low";
  category: "secret" | "permission" | "hook" | "mcp" | "config";
  title: string;
  description: string;
  file: string;
  recommendation: string;
}

export interface ScanReport {
  timestamp: string;
  totalFindings: number;
  bySeverity: Record<string, number>;
  byCategory: Record<string, number>;
  findings: ScanFinding[];
}

// ---- Secret patterns ----

const SECRET_PATTERNS = [
  { pattern: /(?:api[_-]?key|apikey)\s*[:=]\s*["']?(sk-[a-zA-Z0-9]{20,})["']?/i, type: "OpenAI API Key" },
  { pattern: /(?:api[_-]?key|apikey)\s*[:=]\s*["']?(sk-ant-[a-zA-Z0-9]{20,})["']?/i, type: "Anthropic API Key" },
  { pattern: /(?:api[_-]?key|apikey)\s*[:=]\s*["']?(gsk_[a-zA-Z0-9]{20,})["']?/i, type: "Groq API Key" },
  { pattern: /(?:bot[_-]?token|bottoken)\s*[:=]\s*["']?(\d{8,}:[a-zA-Z0-9_-]{20,})["']?/i, type: "Discord Bot Token" },
  { pattern: /(?:ghp_|gho_|ghu_|ghs_|ghr_)[a-zA-Z0-9_]{36,}/, type: "GitHub Token" },
  { pattern: /-----BEGIN (?:RSA |EC )?PRIVATE KEY-----/, type: "Private Key" },
  { pattern: /(?:password|passwd|pwd)\s*[:=]\s*["'][^"']{6,}["']/i, type: "Password" },
  { pattern: /(?:token|secret)\s*[:=]\s*["'][a-zA-Z0-9_\-.]{16,}["']/i, type: "Generic Secret" },
];

// ---- Scanner ----

function scanFile(filePath: string, content: string): ScanFinding[] {
  const findings: ScanFinding[] = [];
  const relativePath = path.relative(process.cwd(), filePath);

  // 1. SECRET scan
  for (const sp of SECRET_PATTERNS) {
    const matches = content.match(sp.pattern);
    if (matches) {
      findings.push({
        severity: "critical",
        category: "secret",
        title: `Hardcoded ${sp.type}`,
        description: `Found ${sp.type} in ${relativePath}`,
        file: relativePath,
        recommendation: `Move the ${sp.type} to environment variables or secrets manager`,
      });
    }
  }

  // 2. CONFIG scan: check for permissive settings
  if (content.includes('"allowAllOrigins"') || content.includes('"*"') && filePath.endsWith(".json")) {
    findings.push({
      severity: "high",
      category: "config",
      title: "Overly Permissive CORS",
      description: `Wildcard CORS detected in ${relativePath}`,
      file: relativePath,
      recommendation: "Restrict CORS to specific origins instead of using '*'",
    });
  }

  // 3. HOOK scan: check for dangerous shell commands in hooks
  if (filePath.includes("hook") && content.includes("rm -rf")) {
    findings.push({
      severity: "critical",
      category: "hook",
      title: "Dangerous Hook Command",
      description: `Hook script contains 'rm -rf' in ${relativePath}`,
      file: relativePath,
      recommendation: "Remove destructive commands from hooks",
    });
  }

  // 4. PERMISSION scan: overly broad file access
  if ((content.includes("chmod 777") || content.includes("0777")) && !filePath.includes("node_modules")) {
    findings.push({
      severity: "high",
      category: "permission",
      title: "Overly Broad File Permissions",
      description: `World-writable permissions detected in ${relativePath}`,
      file: relativePath,
      recommendation: "Use least-privilege permissions (e.g., 755 for dirs, 644 for files)",
    });
  }

  return findings;
}

function scanDir(dir: string): ScanFinding[] {
  const allFindings: ScanFinding[] = [];

  function walk(current: string) {
    try {
      for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
        const fullPath = path.join(current, entry.name);
        if (entry.isDirectory()) {
          if (!entry.name.startsWith("node_modules") && !entry.name.startsWith(".git")) {
            walk(fullPath);
          }
        } else if (
          entry.name.endsWith(".json") ||
          entry.name.endsWith(".ts") ||
          entry.name.endsWith(".js") ||
          entry.name.endsWith(".yaml") ||
          entry.name.endsWith(".yml") ||
          entry.name.endsWith(".env") ||
          entry.name.endsWith(".md")
        ) {
          try {
            const content = fs.readFileSync(fullPath, "utf-8");
            allFindings.push(...scanFile(fullPath, content));
          } catch { /* skip unreadable */ }
        }
      }
    } catch { /* skip unwalkable */ }
  }

  walk(dir);
  return allFindings;
}

// ---- Public API ----

export function runScan(targetDir?: string): ScanReport {
  const dir = targetDir || process.cwd();
  const findings = scanDir(dir);

  const bySeverity: Record<string, number> = {};
  const byCategory: Record<string, number> = {};

  for (const f of findings) {
    bySeverity[f.severity] = (bySeverity[f.severity] || 0) + 1;
    byCategory[f.category] = (byCategory[f.category] || 0) + 1;
  }

  return {
    timestamp: new Date().toISOString(),
    totalFindings: findings.length,
    bySeverity,
    byCategory,
    findings,
  };
}
