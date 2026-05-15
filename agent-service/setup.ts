/**
 * MemFlow Setup Wizard v2 — interactive CLI onboarding
 *
 * Usage:
 *   cd agent-service && npm run setup
 *   # or:  npx tsx setup.ts
 *
 * Like openclaw onboard / hermes setup, but for MemFlow.
 */

import * as readline from "node:readline";
import * as fs from "node:fs";
import * as path from "node:path";
import { execSync, spawn } from "node:child_process";

const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
const q = (p: string): Promise<string> => new Promise((r) => rl.question(p, r));

const C = "\x1b[36m", G = "\x1b[32m", Y = "\x1b[33m", R = "\x1b[31m", X = "\x1b[0m", B = "\x1b[1m", D = "\x1b[2m";
const log = (m: string, c = "") => console.log(`${c}${m}${X}`);
const step = (m: string) => log(`\n>>> ${m}`, C);
const ok = (m: string) => log(`  ${G}✓${X} ${m}`);
const warn = (m: string) => log(`  ${Y}⚠${X} ${m}`);
const fail = (m: string) => { log(`  ${R}✗${X} ${m}`, R); process.exit(1); };

const ROOT = process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(process.cwd(), "..", ".memflow-runtime");
const CONFIG_DIR = path.resolve(ROOT, "config");
const PROVIDERS_PATH = path.resolve(CONFIG_DIR, "providers.json");
const ENV_PATH = path.resolve(process.cwd(), "..", ".env");

const PROVIDER_PRESETS: Record<string, { label: string; defaultModel: string }> = {
  openai: { label: "OpenAI", defaultModel: "gpt-4o-mini" },
  anthropic: { label: "Anthropic", defaultModel: "claude-sonnet-4-20250514" },
  groq: { label: "Groq (fast/cheap)", defaultModel: "llama-3.3-70b-versatile" },
  deepseek: { label: "DeepSeek (cheap)", defaultModel: "deepseek-chat" },
  gemini: { label: "Google Gemini", defaultModel: "gemini-2.0-flash" },
  openrouter: { label: "OpenRouter", defaultModel: "auto" },
  ollama: { label: "Ollama (local)", defaultModel: "llama3.2" },
};

// ─── CHECK ─────────────────────────────────────────────

async function checkPrereqs() {
  step("1/5  Checking prerequisites");
  let ok_ = true;

  try { const v = execSync("node --version", { encoding: "utf8" }).trim(); ok(`Node.js ${v}`); }
  catch { warn("Node.js not found → https://nodejs.org"); ok_ = false; }

  try { execSync("npx --version", { encoding: "utf8" }); ok("npx available"); }
  catch { warn("npx not available, run: npm install -g npx"); ok_ = false; }

  try { execSync("docker --version", { encoding: "utf8" }).trim(); ok("Docker installed"); }
  catch { /* optional */ }

  return ok_;
}

// ─── BUILD ─────────────────────────────────────────────

async function ensureBuilt() {
  const dist = path.resolve(process.cwd(), "dist", "index.js");
  if (fs.existsSync(dist)) { ok("Agent service already built"); return; }
  step("Building agent service...");
  execSync("npm install", { cwd: process.cwd(), stdio: "pipe" });
  execSync("npx tsc --skipLibCheck", { cwd: process.cwd(), stdio: "inherit" });
  ok("Build complete");
}

// ─── START ─────────────────────────────────────────────

async function startAgent(): Promise<boolean> {
  const alive = await fetch("http://localhost:3000/health").then((r) => r.ok).catch(() => false);
  if (alive) { ok("Agent service already running on http://localhost:3000"); return true; }

  const a = await q("\nStart agent service now? (Y/n): ");
  if (a.toLowerCase() === "n") { warn("Start manually later: cd agent-service && npm start"); return false; }

  step("2/5  Starting agent service");
  const p = spawn("node", ["dist/index.js"], {
    cwd: process.cwd(), stdio: "pipe",
    env: { ...process.env, MEMFLOW_RUNTIME_ROOT: ROOT },
    detached: true,
  });
  let out = "";
  p.stdout?.on("data", (d) => { out += d.toString(); });
  p.stderr?.on("data", (d) => { out += d.toString(); });

  // Wait up to 20s
  for (let i = 0; i < 20; i++) {
    await new Promise((r) => setTimeout(r, 1000));
    if (await fetch("http://localhost:3000/health").then((r) => r.ok).catch(() => false)) {
      ok(`Agent service running on http://localhost:3000`);
      return true;
    }
  }
  warn("Service started but not responding yet. Check: cd agent-service && npm start");
  return false;
}

// ─── PROVIDER MENU ─────────────────────────────────────

async function configureProviders() {
  step("3/5  LLM Provider");

  // Check if already configured
  const existing = loadConfig().providers;
  if (existing.length > 0) {
    ok(`${existing.length} provider(s) already configured:`);
    existing.forEach((p) => log(`     ${p.id} (${p.model})`));
    const a = await q("\nReconfigure? (y/N): ");
    if (a.toLowerCase() !== "y") return;
  }

  // Scan .env
  const envKeys = scanEnvFile();
  if (envKeys.length > 0) {
    log(`\n  ${Y}Found API keys in .env:${X}`, Y);
    envKeys.forEach((k) => log(`     ${k.envVar} → ${mask(k.value)}`));
    const a = await q("  Import these providers? (Y/n): ");
    if (a.toLowerCase() !== "n") {
      const providers = envKeys.map((k) => ({
        id: k.providerId, apiKey: k.value, model: PROVIDER_PRESETS[k.providerId]?.defaultModel || "auto", enabled: true,
      }));
      saveConfig({ providers, channels: loadConfig().channels });
      ok(`Imported ${providers.length} provider(s) from .env`);
      return;
    }
  }

  // Interactive menu
  log("\nAvailable providers:");
  const ids = Object.keys(PROVIDER_PRESETS);
  ids.forEach((id, i) => log(`  ${i + 1}. ${PROVIDER_PRESETS[id].label} (${id})`));
  log("  a. Custom provider (any OpenAI-compatible endpoint)");
  log("  0. Done\n");

  const providers: { id: string; apiKey: string; model: string; enabled: boolean }[] = [];

  while (true) {
    const choice = await q("Select provider by number (or 0 to finish): ");
    if (!choice || choice === "0") break;

    const idx = parseInt(choice) - 1;
    if (idx >= 0 && idx < ids.length) {
      const id = ids[idx];
      const preset = PROVIDER_PRESETS[id];
      const apiKey = await q(`  API key for ${preset.label}: `);
      if (!apiKey) { warn("Skipped"); continue; }
      const model = await q(`  Model (default: ${preset.defaultModel}): `) || preset.defaultModel;
      providers.push({ id, apiKey, model, enabled: true });
      ok(`Added ${preset.label}`);
    } else if (choice === "a") {
      log(`  ${D}Custom OpenAI-compatible provider${X}`);
      const id = await q(`  Provider name (e.g. my-llm): `);
      if (!id) continue;
      const baseUrl = await q(`  Base URL (e.g. http://localhost:11434/v1): `);
      const apiKey = await q(`  API key (optional): `);
      const model = await q(`  Model name (e.g. llama3.2): `);
      providers.push({ id, apiKey, model, enabled: true });
      ok(`Added custom provider: ${id}`);
    }
  }

  if (providers.length === 0) { warn("No providers configured"); return; }

  const current = loadConfig();
  current.providers = [...current.providers, ...providers];
  saveConfig(current);
  ok(`${providers.length} provider(s) saved`);
  log(`  File: ${PROVIDERS_PATH}`);
}

// ─── CHANNEL CONFIG ────────────────────────────────────

async function configureChannels() {
  step("4/5  Messaging Channels (optional)");

  const channels = [
    { id: "telegram", fields: [{ key: "botToken", label: "Bot token (from @BotFather)" }] },
    { id: "discord", fields: [{ key: "botToken", label: "Bot token (from Developer Portal)" }] },
    { id: "slack", fields: [{ key: "token", label: "Bot token (xoxb-...)" }] },
    { id: "whatsapp", fields: [{ key: "token", label: "Access token" }, { key: "phoneNumberId", label: "Phone number ID" }] },
    { id: "signal", fields: [{ key: "signalApiUrl", label: "Signal API URL" }] },
  ];

  const a = await q("Configure a messaging channel? (y/N): ");
  if (a.toLowerCase() !== "y") { ok("Skipped (add later via API)"); return; }

  log("\nAvailable channels:");
  channels.forEach((c, i) => log(`  ${i + 1}. ${c.id}`));

  const choice = await q("Select channel by number: ");
  const idx = parseInt(choice);
  if (isNaN(idx) || idx < 1 || idx > channels.length) { warn("Invalid selection"); return; }

  const ch = channels[idx - 1];
  if (!ch) { warn("Channel not found"); return; }

  const config: Record<string, string> = {};
  for (const f of (ch.fields || [])) {
    config[f.key] = await q(`  ${f.label}: `) || "";
  }

  const answer = await q(`  Enable ${ch.id} now? (Y/n): `);
  const enabled = answer.toLowerCase() !== "n";

  const current = loadConfig();
  current.channels.push({ id: ch.id, label: ch.id, enabled, config });
  saveConfig(current);
  ok(`${ch.id} configured and ${enabled ? "enabled" : "disabled"}`);
}

// ─── VERIFY ────────────────────────────────────────────

async function verifySetup() {
  step("5/5  Verification");
  if (!await fetch("http://localhost:3000/health").then((r) => r.ok).catch(() => false)) {
    warn("Service not running. Verify later with: curl http://localhost:3000/health");
    return;
  }
  const h = await fetch("http://localhost:3000/health").then((r) => r.json());
  ok(`Health: ${h.status}, uptime: ${h.uptime_s}s`);

  const p = await fetch("http://localhost:3000/providers").then((r) => r.json()).catch(() => ({}));
  if (p.providers?.length > 0) ok(`${p.providers.length} provider(s) configured`);

  const s = await fetch("http://localhost:3000/skills").then((r) => r.json()).catch(() => ({}));
  if (s.skills?.length > 0) ok(`${s.skills.length} skills loaded`);

  // Quick agent test
  const r = await fetch("http://localhost:3000/agent/execute", {
    method: "POST", headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ text: "hello", stream: false }),
  }).then((r) => r.json()).catch(() => ({}));
  if (r.success) ok(`Agent test: ${r.output?.slice(0, 80)}...`);
  else if (r.error) warn(`Agent test: ${r.error}`);
}

// ─── HELPERS ───────────────────────────────────────────

function loadConfig(): { providers: any[]; channels: any[] } {
  try { const raw = JSON.parse(fs.readFileSync(PROVIDERS_PATH, "utf-8")); return { providers: raw.providers || [], channels: raw.channels || [] }; }
  catch { return { providers: [], channels: [] }; }
}

function saveConfig(cfg: { providers: any[]; channels: any[] }): void {
  if (!fs.existsSync(CONFIG_DIR)) fs.mkdirSync(CONFIG_DIR, { recursive: true });
  fs.writeFileSync(PROVIDERS_PATH, JSON.stringify({ version: 1, ...cfg, updatedAt: new Date().toISOString() }, null, 2), "utf-8");
}

function scanEnvFile(): { envVar: string; providerId: string; value: string }[] {
  try {
    const content = fs.readFileSync(ENV_PATH, "utf-8");
    const map: Record<string, string> = {
      OPENAI_API_KEY: "openai", ANTHROPIC_API_KEY: "anthropic", GROQ_API_KEY: "groq",
      DEEPSEEK_API_KEY: "deepseek", GEMINI_API_KEY: "gemini",
    };
    const found: { envVar: string; providerId: string; value: string }[] = [];
    for (const [envVar, providerId] of Object.entries(map)) {
      const match = content.match(new RegExp(`${envVar}=(.+)`));
      if (match?.[1]?.trim()) found.push({ envVar, providerId, value: match[1].trim() });
    }
    return found;
  } catch { return []; }
}

function mask(s: string): string {
  if (s.length < 8) return s;
  return s.slice(0, 4) + "…" + s.slice(-4);
}

// ─── MAIN ──────────────────────────────────────────────

async function main() {
  console.log(`${B}${C}╔══════════════════════════════════════╗${X}`);
  console.log(`${B}${C}║      MemFlow Setup Wizard v2         ║${X}`);
  console.log(`${B}${C}║    Like openclaw onboard, but for     ║${X}`);
  console.log(`${B}${C}║    your own AI agent platform.        ║${X}`);
  console.log(`${B}${C}╚══════════════════════════════════════╝${X}\n`);

  if (!await checkPrereqs()) { fail("Fix prerequisites and re-run"); }
  await ensureBuilt();
  const running = await startAgent();
  if (running) { await configureProviders(); await configureChannels(); await verifySetup(); }

  const port = 3000;
  log(`\n${B}${G}✅ Setup complete!${X}`, G);
  log(`${B}  Agent service:${X}  http://localhost:${port}`);
  log(`${B}  Health check:${X}  curl http://localhost:${port}/health`);
  log(`${B}  Run a task:${X}   curl -X POST http://localhost:${port}/agent/execute \\`);
  log(`                      -H "Content-Type: application/json" \\`);
  log(`                      -d '{"text":"hello"}'`);
  log(`\n${B}More:${X}`);
  log(`  Skills:       curl http://localhost:${port}/skills`);
  log(`  Curator:      curl -X POST http://localhost:${port}/curator/run`);
  log(`  Middleware:   curl http://localhost:${port}/middleware/config`);
  log(`  Metrics:      curl http://localhost:${port}/metrics`);
  log(`  Channels:     curl http://localhost:${port}/channels`);

  // ─── Optional: Web UI ───────────────────────────────
  const webUiDir = path.resolve(process.cwd(), "..", "web-ui");
  if (fs.existsSync(webUiDir)) {
    const buildWebUi = await q(`\nBuild Web UI (React dashboard)? (y/N): `);
    if (buildWebUi.toLowerCase() === "y") {
      step("Building Web UI");
      try {
        execSync("npm install", { cwd: webUiDir, stdio: "inherit", timeout: 120000 });
        execSync("npx vite build", { cwd: webUiDir, stdio: "inherit", timeout: 120000 });
        ok("Web UI built — available at http://localhost:3000");
      } catch (e: any) {
        warn(`Web UI build failed: ${e.message}`);
        log(`  ${D}Build manually: cd web-ui && npm install && npm run build${X}`);
      }
    }
  }

  log(`\n${Y}Need help?${X}  README.md  |  DEPLOYMENT_GUIDE.md\n`);

  rl.close();
}

main().catch((e) => { console.error(e); process.exit(1); });
