#!/usr/bin/env node

/**
 * @memflow/cli — Standalone MemFlow installer
 *
 * Like `openclaw onboard` / `hermes setup`:
 *   1. Ask where to install
 *   2. Download (git clone or tarball)
 *   3. Build agent-service
 *   4. Run interactive setup wizard
 *   5. Start the service
 *
 * Usage:
 *   npx @memflow/cli
 *   npm i -g @memflow/cli && memflow
 */

import * as readline from "node:readline";
import * as fs from "node:fs";
import * as path from "node:path";
import { execSync, spawn } from "node:child_process";
import * as https from "node:https";
import * as os from "node:os";

// ─── Terminal helpers ─────────────────────────────────

const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
const q = (prompt: string): Promise<string> => new Promise((r) => rl.question(prompt, r));

const C = "\x1b[36m", G = "\x1b[32m", Y = "\x1b[33m", R = "\x1b[31m", M = "\x1b[35m", X = "\x1b[0m", B = "\x1b[1m", D = "\x1b[2m";
const log = (m: string, c = "") => console.log(`${c}${m}${X}`);
const step = (m: string) => log(`\n${B}${C}▶ ${m}${X}`);
const ok = (m: string) => log(`  ${G}✓${X} ${m}`);
const warn = (m: string) => log(`  ${Y}⚠${X} ${m}`);
const fail = (m: string) => { log(`  ${R}✗${X} ${m}`); process.exit(1); };

// ─── Banner ────────────────────────────────────────────

function showBanner() {
  const banner = [
    `${C}${B}`,
    `          _╲_╱_`,
    `         (  •  •  )    ${X}${B}MemFlow${X}${D} — AI Agent Platform${X}`,
    `        (   [M]   )   ${C}${B}╲${X}  ${D}workflow engine + agent runtime${X}`,
    `         ╲ ═══ ╱    ${C}${B} ╲${X} ${D}13 channels · self-learning skills${X}`,
    `       ~~╱╲╱╲╱╲╱╲~~ ${C}${B}  ╱${X}`,
    `       ~~╱╲╱╲╱╲╱╲~~  ${C}${B} ╱${X}`,
    `      ~ ╱╲╱╲╱╲╱╲╱╲~${X}`,
    ``,
    `${D}Memories flow like water.${X}`,
    ``,
  ].join("\n");
  console.log(banner);
}

// ─── Prerequisites ─────────────────────────────────────

function checkPrereqs(): boolean {
  step("Checking prerequisites");
  let ok_ = true;

  try {
    const v = execSync("node --version", { encoding: "utf8", timeout: 5000 }).trim();
    ok(`Node.js ${v}`);
  } catch {
    warn("Node.js not found → https://nodejs.org");
    ok_ = false;
  }

  try {
    execSync("npm --version", { encoding: "utf8", timeout: 5000 });
    ok("npm available");
  } catch {
    warn("npm not found");
    ok_ = false;
  }

  try {
    const r = execSync("git --version", { encoding: "utf8", timeout: 5000 }).trim();
    ok(r);
  } catch {
    warn("git not found — will use tarball download (slower)");
  }

  try {
    const d = execSync("docker --version", { encoding: "utf8", timeout: 5000 }).trim();
    ok(`${d} (optional)`);
  } catch {
    /* optional */
  }

  return ok_;
}

// ─── Pick install dir ─────────────────────────────────

async function pickInstallDir(): Promise<string> {
  const defaultDir = path.resolve(process.cwd(), "memflow");
  step("Where to install MemFlow?");

  log(`  ${D}Enter path (default: ${defaultDir})${X}`);
  const dir = await q(`  ${B}>${X} `);
  const resolved = dir.trim() ? path.resolve(process.cwd(), dir.trim()) : defaultDir;

  if (fs.existsSync(resolved)) {
    const items = fs.readdirSync(resolved);
    if (items.length > 0) {
      const a = await q(`  ${Y}"${resolved}" already exists and is not empty.${X}\n    Overwrite? (y/N): `);
      if (a.toLowerCase() !== "y") {
        log(`  ${D}Exiting. Pick a different directory.${X}`);
        process.exit(0);
      }
    }
  }

  ok(`Install target: ${resolved}`);
  return resolved;
}

// ─── Download ──────────────────────────────────────────

async function downloadRepo(targetDir: string): Promise<void> {
  step("Downloading MemFlow");

  // Try git clone first (fast)
  try {
    execSync("git --version", { encoding: "utf8", timeout: 5000 });

    if (!fs.existsSync(targetDir)) {
      fs.mkdirSync(targetDir, { recursive: true });
    }
    // Only clone if dir is empty
    if (fs.readdirSync(targetDir).length === 0) {
      log(`  ${D}Cloning from GitHub (shallow)...${X}`);
      execSync(
        `git clone --depth 1 https://github.com/Strue1994/MemFlow.git "${targetDir}"`,
        { stdio: "inherit", timeout: 120000 }
      );
      ok("Downloaded via git");
      return;
    }
    // Dir exists with content — git pull instead
    log(`  ${D}Pulling latest...${X}`);
    execSync(`git pull`, { cwd: targetDir, stdio: "inherit", timeout: 60000 });
    ok("Updated via git pull");
    return;
  } catch {
    /* git not available or failed, fall through to tarball */
  }

  // Fallback: tarball download
  log(`  ${D}Downloading tarball...${X}`);
  const tarballUrl = "https://github.com/Strue1994/MemFlow/archive/refs/heads/master.tar.gz";
  const tmpFile = path.join(os.tmpdir(), `memflow-${Date.now()}.tar.gz`);

  await downloadFile(tarballUrl, tmpFile);

  // Extract
  fs.mkdirSync(targetDir, { recursive: true });
  execSync(
    `tar -xzf "${tmpFile}" -C "${targetDir}" --strip-components=1`,
    { stdio: "inherit", timeout: 30000 }
  );
  fs.unlinkSync(tmpFile);
  ok("Downloaded via tarball");
}

function downloadFile(url: string, dest: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (res) => {
      // Follow redirects
      if (res.statusCode && res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        file.close();
        fs.unlinkSync(dest);
        return downloadFile(res.headers.location, dest).then(resolve).catch(reject);
      }
      if (res.statusCode !== 200) {
        file.close();
        fs.unlinkSync(dest);
        return reject(new Error(`HTTP ${res.statusCode}`));
      }
      res.pipe(file);
      file.on("finish", () => { file.close(); resolve(); });
    }).on("error", (err) => {
      file.close();
      try { fs.unlinkSync(dest); } catch { /* ignore */ }
      reject(err);
    });
  });
}

// ─── Build ─────────────────────────────────────────────

function buildAgentService(agentDir: string): void {
  step("Building agent service");

  const distPath = path.join(agentDir, "dist", "index.js");
  if (fs.existsSync(distPath)) {
    ok("Already built");
    return;
  }

  log(`  ${D}Installing dependencies...${X}`);
  execSync("npm install", { cwd: agentDir, stdio: "inherit", timeout: 120000 });
  ok("Dependencies installed");

  log(`  ${D}Compiling TypeScript...${X}`);
  execSync("npx tsc --skipLibCheck", { cwd: agentDir, stdio: "inherit", timeout: 120000 });
  ok("Build complete");
}

// ─── Run setup wizard ─────────────────────────────────

async function runSetupWizard(agentDir: string): Promise<void> {
  step("Running setup wizard");

  log(`  ${D}Configure providers, channels, and more...${X}\n`);

  const setupScript = path.join(agentDir, "setup.ts");

  // Spawn setup wizard as child — inherits stdin/stdout so user sees the same interactive UX
  return new Promise((resolve, reject) => {
    const child = spawn("npx", ["tsx", setupScript], {
      cwd: agentDir,
      stdio: "inherit",
      env: { ...process.env, MEMFLOW_RUNTIME_ROOT: path.resolve(agentDir, "..", ".memflow-runtime") },
      shell: process.platform === "win32",
    });
    child.on("exit", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`Setup wizard exited with code ${code}`));
    });
    child.on("error", reject);
  });
}

// ─── Summary ───────────────────────────────────────────

function showSummary(installDir: string): void {
  const agentDir = path.join(installDir, "agent-service");

  log(`\n${B}${G}╔═══════════════════════════════════════════╗${X}`);
  log(`${B}${G}║        MemFlow is ready!                  ║${X}`);
  log(`${B}${G}╚═══════════════════════════════════════════╝${X}`);
  log(``);
  log(`  ${B}Installed at:${X}  ${installDir}`);
  log(`  ${B}Agent service:${X} http://localhost:3000`);
  log(``);
  log(`  ${B}Start manually:${X}`);
  log(`    cd "${agentDir}"`);
  log(`    npm start`);
  log(``);
  log(`  ${B}Check health:${X}`);
  log(`    curl http://localhost:3000/health`);
  log(``);
  log(`  ${B}Run a task:${X}`);
  log(`    curl -X POST http://localhost:3000/agent/execute \\`);
  log(`      -H "Content-Type: application/json" \\`);
  log(`      -d '{"text":"hello"}'`);
  log(``);
  log(`  ${B}Docker (full stack):${X}`);
  log(`    cd "${installDir}"`);
  log(`    docker compose up -d`);
  log(``);
  log(`  ${B}Docs:${X}  ${installDir}/README.md`);
  log(``);
  log(`${D}  Report issues → https://github.com/Strue1994/MemFlow/issues${X}`);
  log(``);
}

// ─── Main ──────────────────────────────────────────────

async function main() {
  showBanner();

  const ok_ = checkPrereqs();
  if (!ok_) {
    const a = await q(`\nMissing prerequisites. Continue anyway? (y/N): `);
    if (a.toLowerCase() !== "y") { log("Aborted."); process.exit(1); }
  }

  const installDir = await pickInstallDir();
  await downloadRepo(installDir);

  const agentDir = path.join(installDir, "agent-service");
  if (!fs.existsSync(agentDir)) {
    fail(`Expected directory not found: ${agentDir}`);
  }

  buildAgentService(agentDir);

  try {
    await runSetupWizard(agentDir);
  } catch (e: any) {
    warn(`Setup wizard: ${e.message}`);
    log(`  ${D}You can re-run it later: cd "${agentDir}" && npm run setup${X}`);
  }

  showSummary(installDir);
  rl.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
