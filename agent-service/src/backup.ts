import * as fs from "node:fs";
import * as path from "node:path";

function runtimeRoot(): string {
  return process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(process.cwd(), "..", ".memflow-runtime");
}

function backupsRoot(): string {
  return path.resolve(runtimeRoot(), "backups");
}

function copyRecursive(src: string, dst: string): void {
  if (!fs.existsSync(src)) return;
  const stat = fs.statSync(src);
  if (stat.isDirectory()) {
    if (!fs.existsSync(dst)) fs.mkdirSync(dst, { recursive: true });
    for (const name of fs.readdirSync(src)) {
      copyRecursive(path.join(src, name), path.join(dst, name));
    }
  } else {
    fs.copyFileSync(src, dst);
  }
}

export function createBackup(): { path: string } {
  const ts = new Date().toISOString().replace(/[:.]/g, "-");
  const dst = path.resolve(backupsRoot(), ts);
  if (!fs.existsSync(backupsRoot())) fs.mkdirSync(backupsRoot(), { recursive: true });
  copyRecursive(runtimeRoot(), dst);
  return { path: dst };
}

export function restoreBackup(backupPath: string): { restored: boolean } {
  if (!fs.existsSync(backupPath)) throw new Error("backup path not found");
  copyRecursive(backupPath, runtimeRoot());
  return { restored: true };
}
