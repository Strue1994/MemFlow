import { Router, type Express, type Request, type Response, type NextFunction } from "express";
import { promises as fs } from "node:fs";
import path from "node:path";
import os from "node:os";
import { execFile } from "node:child_process";

const COMPUTER_ROOT = path.resolve(__dirname, "..", "..");
const DEFAULT_COMMAND_TIMEOUT_MS = Number(process.env.COMPUTER_AGENT_TIMEOUT_MS || "15000");
const MAX_FILE_READ_BYTES = Number(process.env.COMPUTER_AGENT_MAX_READ_BYTES || "131072");
const MAX_FILE_WRITE_BYTES = Number(process.env.COMPUTER_AGENT_MAX_WRITE_BYTES || "262144");
const MAX_FILE_SEARCH_RESULTS = Number(process.env.COMPUTER_AGENT_MAX_SEARCH_RESULTS || "80");
const MAX_FILE_SEARCH_DEPTH = Number(process.env.COMPUTER_AGENT_MAX_SEARCH_DEPTH || "6");
const BROWSER_AUTOMATION_AVAILABLE = false;
const SEARCH_SKIP_DIR_NAMES = new Set([
  ".git",
  ".github",
  "node_modules",
  "target",
  "dist",
  "build",
  ".npm-cache",
  "logs",
]);

const ALLOWED_COMMAND_PATTERNS = [
  /^dir\b/i,
  /^type\b/i,
  /^netstat\b/i,
  /^git\s+(status|diff|log|branch)\b/i,
  /^npm\s+(run\s+[a-z0-9:_-]+|test|build)\b/i,
  /^cargo\s+(build|check|test|run)\b/i,
  /^python(?:\.exe)?\s+/i,
  /^node(?:\.exe)?\s+/i,
];

const DANGEROUS_COMMAND_PATTERN =
  /\b(remove-item|del|erase|rd|rmdir|format|shutdown|restart-computer|stop-computer|taskkill|reg\s+delete|sc\s+delete|move-item|rename-item|set-acl|icacls|takeown|cipher|bcdedit|diskpart)\b/i;

function normalizeCommand(command: string): string | null {
  const trimmed = command.trim();

  if (/^Get-ChildItem\b/i.test(trimmed)) {
    if (/\s-Name\b/i.test(trimmed)) {
      return "dir /b";
    }
    if (/\s-Force\b/i.test(trimmed)) {
      return "dir /a";
    }
    return "dir";
  }

  const getContentMatch = trimmed.match(/^Get-Content\s+(.+)$/i);
  if (getContentMatch) {
    return `type ${getContentMatch[1]}`;
  }

  if (/^(Set-Content|Add-Content|New-Item)\b/i.test(trimmed)) {
    return null;
  }

  return trimmed;
}

export interface CommandResult {
  allowed: boolean;
  command: string;
  cwd: string;
  stdout: string;
  stderr: string;
  exitCode: number | null;
  durationMs: number;
}

function commandResult(
  command: string,
  cwd: string,
  stdout: string,
  stderr = "",
  exitCode = 0,
  durationMs = 0,
  allowed = true,
): CommandResult {
  return {
    allowed,
    command,
    cwd,
    stdout: truncate(stdout),
    stderr: truncate(stderr),
    exitCode,
    durationMs,
  };
}

function extractBearerToken(req: Request): string | null {
  const authHeader = req.get("authorization");
  if (authHeader?.startsWith("Bearer ")) {
    return authHeader.slice("Bearer ".length);
  }
  return null;
}

function requireComputerAuth(req: Request, res: Response, next: NextFunction): void {
  const configured = process.env.EXECUTOR_API_KEY;
  if (!configured) {
    next();
    return;
  }

  const token = extractBearerToken(req);
  if (token === configured) {
    next();
    return;
  }

  res.status(401).json({ error: "Missing or invalid API key" });
}

function resolveSafePath(relativePath: string | undefined): string {
  const candidate = path.resolve(COMPUTER_ROOT, relativePath || ".");
  if (!candidate.startsWith(COMPUTER_ROOT)) {
    throw new Error("Path escapes computer agent root");
  }
  return candidate;
}

function truncate(text: string, limit = 12000): string {
  if (text.length <= limit) {
    return text;
  }
  return `${text.slice(0, limit)}\n... [truncated ${text.length - limit} chars]`;
}

async function listDirectory(relativePath?: string) {
  const directory = resolveSafePath(relativePath);
  const entries = await fs.readdir(directory, { withFileTypes: true });
  const items = await Promise.all(
    entries
      .sort((a, b) => Number(b.isDirectory()) - Number(a.isDirectory()) || a.name.localeCompare(b.name))
      .map(async (entry) => {
        const absolutePath = path.join(directory, entry.name);
        const stat = await fs.stat(absolutePath);
        return {
          name: entry.name,
          path: path.relative(COMPUTER_ROOT, absolutePath) || ".",
          kind: entry.isDirectory() ? "directory" : "file",
          size: entry.isDirectory() ? null : stat.size,
          modifiedAt: stat.mtime.toISOString(),
        };
      }),
  );

  return {
    root: COMPUTER_ROOT,
    path: path.relative(COMPUTER_ROOT, directory) || ".",
    items,
  };
}

async function readTextFile(relativePath: string) {
  const filePath = resolveSafePath(relativePath);
  const stat = await fs.stat(filePath);
  if (!stat.isFile()) {
    throw new Error("Path is not a file");
  }
  if (stat.size > MAX_FILE_READ_BYTES) {
    throw new Error(`File exceeds max read size of ${MAX_FILE_READ_BYTES} bytes`);
  }
  const content = await fs.readFile(filePath, "utf8");
  return {
    path: path.relative(COMPUTER_ROOT, filePath),
    size: stat.size,
    content,
  };
}

async function writeTextFile(relativePath: string, content: string, append = false) {
  const filePath = resolveSafePath(relativePath);
  const byteLength = Buffer.byteLength(content, "utf8");
  if (byteLength > MAX_FILE_WRITE_BYTES) {
    throw new Error(`Content exceeds max write size of ${MAX_FILE_WRITE_BYTES} bytes`);
  }
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  if (append) {
    await fs.appendFile(filePath, content, "utf8");
  } else {
    await fs.writeFile(filePath, content, "utf8");
  }
  const stat = await fs.stat(filePath);
  return {
    path: path.relative(COMPUTER_ROOT, filePath),
    size: stat.size,
    modifiedAt: stat.mtime.toISOString(),
    mode: append ? "append" : "write",
  };
}

async function searchFiles(query: string, relativePath?: string) {
  const trimmedQuery = query.trim().toLowerCase();
  if (trimmedQuery.length < 2) {
    throw new Error("Search query must be at least 2 characters");
  }

  const rootDirectory = resolveSafePath(relativePath);
  const results: Array<{
    name: string;
    path: string;
    kind: "file" | "directory";
    size: number | null;
    modifiedAt: string;
  }> = [];
  const pending: Array<{ directory: string; depth: number }> = [{ directory: rootDirectory, depth: 0 }];

  while (pending.length > 0 && results.length < MAX_FILE_SEARCH_RESULTS) {
    const current = pending.pop();
    if (!current) {
      break;
    }

    const entries = await fs.readdir(current.directory, { withFileTypes: true });
    entries.sort((a, b) => Number(b.isDirectory()) - Number(a.isDirectory()) || a.name.localeCompare(b.name));

    for (const entry of entries) {
      if (results.length >= MAX_FILE_SEARCH_RESULTS) {
        break;
      }

      const absolutePath = path.join(current.directory, entry.name);
      const relativeEntryPath = path.relative(COMPUTER_ROOT, absolutePath) || ".";

      if (entry.name.toLowerCase().includes(trimmedQuery)) {
        const stat = await fs.stat(absolutePath);
        results.push({
          name: entry.name,
          path: relativeEntryPath,
          kind: entry.isDirectory() ? "directory" : "file",
          size: entry.isDirectory() ? null : stat.size,
          modifiedAt: stat.mtime.toISOString(),
        });
      }

      if (
        entry.isDirectory() &&
        current.depth < MAX_FILE_SEARCH_DEPTH &&
        !SEARCH_SKIP_DIR_NAMES.has(entry.name.toLowerCase())
      ) {
        pending.push({ directory: absolutePath, depth: current.depth + 1 });
      }
    }
  }

  return {
    root: COMPUTER_ROOT,
    path: path.relative(COMPUTER_ROOT, rootDirectory) || ".",
    query,
    limit: MAX_FILE_SEARCH_RESULTS,
    truncated: results.length >= MAX_FILE_SEARCH_RESULTS,
    items: results,
  };
}

function isCommandAllowed(command: string): boolean {
  if (DANGEROUS_COMMAND_PATTERN.test(command)) {
    return false;
  }
  return ALLOWED_COMMAND_PATTERNS.some((pattern) => pattern.test(command.trim()));
}

async function runCommand(command: string, cwd?: string): Promise<CommandResult> {
  const workingDirectory = resolveSafePath(cwd);
  const start = Date.now();
  const normalized = normalizeCommand(command);

  if (!normalized || !isCommandAllowed(normalized)) {
    return {
      allowed: false,
      command,
      cwd: path.relative(COMPUTER_ROOT, workingDirectory) || ".",
      stdout: "",
      stderr: "Command is blocked by safe-mode policy",
      exitCode: null,
      durationMs: Date.now() - start,
    };
  }

  if (/^dir\b/i.test(normalized)) {
    const bare = /\s\/b\b/i.test(normalized);
    const parts = normalized.split(/\s+/).slice(1).filter((part) => !part.startsWith('/'));
    const target = parts.join(' ').trim() || path.relative(COMPUTER_ROOT, workingDirectory) || '.';
    const listing = await listDirectory(target);
    const stdout = bare
      ? listing.items.map((item) => item.name).join(os.EOL)
      : listing.items
          .map((item) => `${item.kind === 'directory' ? '[DIR] ' : '[FILE]'} ${item.path}`)
          .join(os.EOL);
    return commandResult(normalized, listing.path, stdout, "", 0, Date.now() - start);
  }

  if (/^type\b/i.test(normalized)) {
    const fileTarget = normalized.replace(/^type\s+/i, "").trim();
    if (!fileTarget) {
      return commandResult(normalized, path.relative(COMPUTER_ROOT, workingDirectory) || ".", "", "Missing file path", 1, Date.now() - start);
    }
    const file = await readTextFile(fileTarget);
    return commandResult(normalized, path.relative(COMPUTER_ROOT, workingDirectory) || ".", file.content, "", 0, Date.now() - start);
  }

  const result = await new Promise<CommandResult>((resolve) => {
    let settled = false;
    const finish = (value: CommandResult) => {
      if (!settled) {
        settled = true;
        resolve(value);
      }
    };

    try {
      const child = execFile(
        process.env.ComSpec || "cmd.exe",
        ["/d", "/s", "/c", normalized],
        {
          cwd: workingDirectory,
          timeout: DEFAULT_COMMAND_TIMEOUT_MS,
          windowsHide: true,
          maxBuffer: 1024 * 1024,
        },
        (error, stdout, stderr) => {
          if (error && /EPERM/i.test(error.message || "")) {
            finish(
              commandResult(
                normalized,
                path.relative(COMPUTER_ROOT, workingDirectory) || ".",
                "",
                "This environment blocks subprocess execution (spawn EPERM). Use filesystem/browser actions or built-in dir/type commands instead.",
                1,
                Date.now() - start,
                false,
              ),
            );
            return;
          }
          finish({
            allowed: true,
            command: normalized,
            cwd: path.relative(COMPUTER_ROOT, workingDirectory) || ".",
            stdout: truncate(stdout ?? ""),
            stderr: truncate(stderr ?? error?.message ?? ""),
            exitCode: error && typeof (error as any).code === "number" ? (error as any).code : (error ? 1 : 0),
            durationMs: Date.now() - start,
          });
        },
      );
      child.once("error", (error) => {
        const message = error instanceof Error ? error.message : String(error);
        finish(
          commandResult(
            normalized,
            path.relative(COMPUTER_ROOT, workingDirectory) || ".",
            "",
            message,
            1,
            Date.now() - start,
            false,
          ),
        );
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      finish(
        commandResult(
          normalized,
          path.relative(COMPUTER_ROOT, workingDirectory) || ".",
          "",
          message,
          1,
          Date.now() - start,
          false,
        ),
      );
    }
  });

  return result;
}

export async function runComputerCommand(command: string, cwd?: string): Promise<CommandResult> {
  return runCommand(command, cwd);
}

function isValidHttpUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

async function openUrl(url: string) {
  if (!isValidHttpUrl(url)) {
    throw new Error("Only http/https URLs are supported");
  }
  await new Promise<void>((resolve, reject) => {
    execFile(
      process.env.ComSpec || "cmd.exe",
      ["/d", "/s", "/c", `start "" "${url}"`],
      { windowsHide: true },
      (error) => {
        if (error) {
          reject(error);
          return;
        }
        resolve();
      },
    );
  });

  return { opened: true, url };
}

async function fetchUrl(url: string) {
  if (!isValidHttpUrl(url)) {
    throw new Error("Only http/https URLs are supported");
  }
  const response = await fetch(url, {
    headers: { "User-Agent": "memflow-computer-agent/1.0" },
  });
  const text = truncate(await response.text(), 20000);
  const titleMatch = text.match(/<title[^>]*>(.*?)<\/title>/i);
  return {
    url,
    status: response.status,
    contentType: response.headers.get("content-type"),
    title: titleMatch?.[1]?.trim() || null,
    bodyPreview: text,
  };
}

export function attachComputerRoutes(app: Express): void {
  const router = Router();
  router.use(requireComputerAuth);

  router.get("/capabilities", (_req, res) => {
    res.json({
      root: COMPUTER_ROOT,
      platform: os.platform(),
      browser: {
        openUrl: true,
        fetchPage: true,
        automation: BROWSER_AUTOMATION_AVAILABLE,
        automationNote: BROWSER_AUTOMATION_AVAILABLE
          ? "Interactive browser automation is available"
          : "Interactive browser automation is not verified in this Windows environment yet",
      },
      filesystem: {
        list: true,
        search: true,
        read: true,
        write: true,
        sandboxRoot: COMPUTER_ROOT,
      },
      terminal: {
        run: true,
        safeMode: true,
        timeoutMs: DEFAULT_COMMAND_TIMEOUT_MS,
        allowPatterns: ALLOWED_COMMAND_PATTERNS.map((pattern) => pattern.toString()),
      },
    });
  });

  router.get("/fs/list", async (req, res) => {
    try {
      res.json(await listDirectory(String(req.query.path || ".")));
    } catch (error) {
      res.status(400).json({ error: error instanceof Error ? error.message : String(error) });
    }
  });

  router.get("/fs/search", async (req, res) => {
    try {
      const query = String(req.query.q || "");
      const relativePath = typeof req.query.path === "string" ? req.query.path : undefined;
      const result = await searchFiles(query, relativePath);
      res.json(result);
    } catch (error: any) {
      res.status(400).json({ error: error.message });
    }
  });

  router.get("/fs/read", async (req, res) => {
    try {
      const relativePath = String(req.query.path || "");
      if (!relativePath) {
        res.status(400).json({ error: "path is required" });
        return;
      }
      res.json(await readTextFile(relativePath));
    } catch (error) {
      res.status(400).json({ error: error instanceof Error ? error.message : String(error) });
    }
  });

  router.post("/fs/write", async (req, res) => {
    try {
      const relativePath = String(req.body?.path || "");
      if (!relativePath) {
        res.status(400).json({ error: "path is required" });
        return;
      }
      const content = String(req.body?.content || "");
      const append = Boolean(req.body?.append);
      res.json(await writeTextFile(relativePath, content, append));
    } catch (error) {
      res.status(400).json({ error: error instanceof Error ? error.message : String(error) });
    }
  });

  router.post("/terminal/run", async (req, res) => {
    try {
      const command = String(req.body?.command || "");
      if (!command.trim()) {
        res.status(400).json({ error: "command is required" });
        return;
      }
      const cwd = req.body?.cwd ? String(req.body.cwd) : undefined;
      res.json(await runCommand(command, cwd));
    } catch (error) {
      res.status(400).json({ error: error instanceof Error ? error.message : String(error) });
    }
  });

  router.post("/browser/open", async (req, res) => {
    try {
      const url = String(req.body?.url || "");
      if (!url) {
        res.status(400).json({ error: "url is required" });
        return;
      }
      res.json(await openUrl(url));
    } catch (error) {
      res.status(400).json({ error: error instanceof Error ? error.message : String(error) });
    }
  });

  router.post("/browser/fetch", async (req, res) => {
    try {
      const url = String(req.body?.url || "");
      if (!url) {
        res.status(400).json({ error: "url is required" });
        return;
      }
      res.json(await fetchUrl(url));
    } catch (error) {
      res.status(400).json({ error: error instanceof Error ? error.message : String(error) });
    }
  });

  app.use("/computer", router);
}
