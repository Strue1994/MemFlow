/**
 * MemFlow Setup Wizard — CLI onboarding like openclaw onboard / hermes setup
 *
 * Usage: npx ts-node setup.ts  or  node dist/setup.js
 *
 * Guides the user through:
 * 1. Checking prerequisites
 * 2. Configuring LLM providers
 * 3. Configuring messaging channels
 * 4. Testing the configuration
 * 5. Starting the service
 */

import * as readline from "node:readline";
import * as fs from "node:fs";
import * as path from "node:path";
import { execSync, spawn } from "node:child_process";

const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
function question(prompt: string): Promise<string> {
  return new Promise((resolve) => rl.question(prompt, resolve));
}

const CYAN = "\x1b[36m";
const GREEN = "\x1b[32m";
const YELLOW = "\x1b[33m";
const RED = "\x1b[31m";
const RESET = "\x1b[0m";
const BOLD = "\x1b[1m";

function log(msg: string, color = "") { console.log(`${color}${msg}${RESET}`); }
function step(msg: string) { log(`\n>>> ${msg}`, CYAN); }
function ok(msg: string) { log(`  [OK] ${msg}`, GREEN); }
function warn(msg: string) { log(`  [!] ${msg}`, YELLOW); }
function fail(msg: string) { log(`  [ERR] ${msg}`, RED); }

const RUNTIME_ROOT = process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(process.cwd(), "..", ".memflow-runtime");
const CONFIG_DIR = path.resolve(RUNTIME_ROOT, "config");
const PROVIDERS_PATH = path.resolve(CONFIG_DIR, "providers.json");
const SKILLS_DIR = path.resolve(RUNTIME_ROOT, "skills");

// ---- Prerequisites ----

async function checkPrerequisites(): Promise<boolean> {
  step("Checking prerequisites");
  let allOk = true;

  // Node.js
  try {
    const nodeVer = execSync("node --version", { encoding: "utf8" }).trim();
    ok(`Node.js ${nodeVer}`);
  } catch { warn("Node.js not found. Install from https://nodejs.org"); allOk = false; }

  // Docker (optional)
  try {
    const dockerVer = execSync("docker --version", { encoding: "utf8" }).trim();
    ok(`Docker ${dockerVer}`);
  } catch { warn("Docker not found (optional — needed for Docker Compose deployment)"); }

  // Git
  try {
    const gitVer = execSync("git --version", { encoding: "utf8" }).trim();
    ok(`Git ${gitVer}`);
  } catch { warn("Git not found (optional)"); }

  // Port 3000
  try {
    const resp = await fetch("http://localhost:3000/health");
    if (resp.ok) warn("Port 3000 already in use — another MemFlow instance may be running");
  } catch { ok("Port 3000 is free"); }

  return allOk;
}

// ---- Provider Configuration ----

async function configureProviders(): Promise<void> {
  step("Configure LLM Providers");
  log("At least one LLM provider is required. Supported providers:", YELLOW);
  log("  openai, anthropic, groq, deepseek, gemini, together, perplexity, xai, mistral, ollama");
  log("(You can add more later via the API)\n");

  const providers: any[] = [];
  const existing = loadProviders();
  if (existing.length > 0) {
    log(`Found ${existing.length} existing provider(s). Skipping setup...`, GREEN);
    return;
  }

  while (true) {
    const id = await question("Enter provider id (e.g. openai) or press Enter to finish: ");
    if (!id) break;

    const apiKey = await question(`  API key for ${id}: `);
    if (!apiKey) { warn("API key required, skipping"); continue; }

    const model = await question(`  Model (default: auto): `) || "auto";

    providers.push({ id, apiKey, model, enabled: true });
    ok(`Added ${id}`);
    log(""); // blank line
  }

  if (providers.length === 0) {
    warn("No providers configured. You can add them later via:");
    log("  curl -X POST http://localhost:3000/providers -d '{\"id\":\"openai\",\"apiKey\":\"sk-...\"}'");
    return;
  }

  saveProviders(providers);
  ok(`${providers.length} provider(s) saved to ${PROVIDERS_PATH}`);
}

function loadProviders(): any[] {
  try { return JSON.parse(fs.readFileSync(PROVIDERS_PATH, "utf-8")).providers || []; }
  catch { return []; }
}

function saveProviders(providers: any[]): void {
  const config = { version: 1, providers, channels: [], updatedAt: new Date().toISOString() };
  if (!fs.existsSync(CONFIG_DIR)) fs.mkdirSync(CONFIG_DIR, { recursive: true });
  fs.writeFileSync(PROVIDERS_PATH, JSON.stringify(config, null, 2), "utf-8");
}

// ---- Channel Configuration ----

async function configureChannels(): Promise<void> {
  step("Configure Messaging Channels (optional)");
  log("Supported channels: telegram, discord, slack, whatsapp, signal, email, matrix, teams, google-chat, line");
  log("You can also configure channels later via API.\n");

  const answer = await question("Configure a channel now? (y/n): ");
  if (answer.toLowerCase() !== "y") {
    ok("Skipping channel configuration");
    return;
  }

  const channel = await question("Channel id (e.g. telegram): ");
  if (!channel) { warn("No channel configured"); return; }
  ok(`To configure ${channel}, run later:`);
  log(`  curl -X POST http://localhost:3000/channels -H "Content-Type: application/json" -d '{"id":"${channel}","enabled":true,"config":{...}}'`);
}

// ---- Service Start ----

async function startService(): Promise<void> {
  step("Starting MemFlow Agent Service");

  const alreadyRunning = await fetch("http://localhost:3000/health").then((r) => r.ok).catch(() => false);
  if (alreadyRunning) {
    ok("Agent service is already running on http://localhost:3000");
    return;
  }

  const distPath = path.resolve(process.cwd(), "dist", "index.js");
  if (!fs.existsSync(distPath)) {
    warn("Building TypeScript...");
    execSync("npx tsc --skipLibCheck", { cwd: process.cwd(), stdio: "inherit" });
    ok("Build complete");
  }

  log("Starting agent service...");
  const proc = spawn("node", ["dist/index.js"], {
    cwd: process.cwd(),
    stdio: "inherit",
    env: { ...process.env, MEMFLOW_RUNTIME_ROOT: RUNTIME_ROOT },
    detached: true,
  });

  // Wait for it to be ready
  for (let i = 0; i < 15; i++) {
    await new Promise((r) => setTimeout(r, 1000));
    const ready = await fetch("http://localhost:3000/health").then((r) => r.ok).catch(() => false);
    if (ready) {
      ok(`Agent service is running on http://localhost:3000 (PID: ${proc.pid})`);
      return;
    }
  }

  warn("Service started but not yet responding. Check logs with: node dist/index.js");
}

// ---- Test ----

async function testConfiguration(): Promise<void> {
  step("Testing Configuration");

  const health = await fetch("http://localhost:3000/health").then((r) => r.json()).catch(() => null);
  if (!health) { fail("Agent service is not running. Start it first."); return; }
  ok(`Health: ${health.status}, uptime: ${health.uptime_s}s`);

  const live = await fetch("http://localhost:3000/live").then((r) => r.json()).catch(() => null);
  if (live?.live) ok("Liveness check passed");

  const providers = await fetch("http://localhost:3000/providers").then((r) => r.json()).catch(() => null);
  if (providers?.providers?.length > 0) ok(`${providers.providers.length} provider(s) configured`);
  else warn("No providers configured. Add one via POST /providers");

  const skills = await fetch("http://localhost:3000/skills").then((r) => r.json()).catch(() => null);
  if (skills?.skills?.length > 0) ok(`${skills.skills.length} skills loaded`);
}

// ---- Main ----

async function main() {
  log(`${BOLD}MemFlow Setup Wizard${RESET}`, CYAN);
  log("This wizard will help you set up MemFlow step by step.\n");

  // 1. Prerequisites
  const prereqsOk = await checkPrerequisites();

  // 2. Agent service must be running for API calls
  const serviceRunning = await fetch("http://localhost:3000/health").then((r) => r.ok).catch(() => false);
  if (!serviceRunning) {
    const start = await question("\nStart the agent service now? (y/n): ");
    if (start.toLowerCase() === "y") {
      await startService();
    } else {
      warn("Service must be running to configure providers. Start it manually:");
      log("  cd agent-service && node dist/index.js");
    }
  }

  // 3. Configure providers
  await configureProviders();

  // 4. Configure channels
  await configureChannels();

  // 5. Test
  if (await fetch("http://localhost:3000/health").then((r) => r.ok).catch(() => false)) {
    await testConfiguration();
  }

  // 6. Summary
  step("Setup Complete");
  log(`
${BOLD}Next Steps:${RESET}
  • API:       http://localhost:3000
  • Health:    curl http://localhost:3000/health
  • Run task:  curl -X POST http://localhost:3000/agent/execute \\
                 -H "Content-Type: application/json" \\
                 -d '{"text":"hello"}'

${BOLD}Documentation:${RESET}
  • README:    ${path.resolve(process.cwd(), "..", "README.md")}
  • Deploy:    ${path.resolve(process.cwd(), "..", "DEPLOYMENT_GUIDE.md")}

${BOLD}Useful Commands:${RESET}
  • View skills:       curl http://localhost:3000/skills
  • View providers:    curl http://localhost:3000/providers
  • Run curator:       curl -X POST http://localhost:3000/curator/run
  • Health check:      curl http://localhost:3000/health
  • Metrics:           curl http://localhost:3000/metrics
`, GREEN);

  rl.close();
}

main().catch((err) => { console.error(err); process.exit(1); });
