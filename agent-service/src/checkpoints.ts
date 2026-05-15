/**
 * Checkpoints v2 — Hermes-aligned session state persistence + auto-resume
 *
 * Features:
 * - Save/restore agent session state (messages, context, iteration)
 * - Automatic pruning (keep last 10, max 1GB total)
 * - Auto-resume on service restart
 * - Per-session isolation
 */

import * as fs from "node:fs";
import * as path from "node:path";

// ---- Types ----

export interface Checkpoint {
  id: string;
  sessionId: string;
  createdAt: string;
  iteration: number;
  totalTokens: number;
  messageCount: number;
  messages: any[];
  summary: string;
}

export interface CheckpointStore {
  checkpoints: Checkpoint[];
  lastSessionId: string | null;
}

// ---- Config ----

const MAX_CHECKPOINTS_PER_SESSION = 10;
const MAX_TOTAL_STORAGE_BYTES = 1_000_000_000; // 1GB

function getStorageDir(): string {
  const root = process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(process.cwd(), "..", ".memflow-runtime");
  return path.resolve(root, "checkpoints");
}

function getStorePath(): string {
  return path.resolve(getStorageDir(), "store.json");
}

// ---- Store I/O ----

function ensureDir(): void {
  const dir = getStorageDir();
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
}

function readStore(): CheckpointStore {
  try {
    const raw = fs.readFileSync(getStorePath(), "utf-8");
    return JSON.parse(raw);
  } catch {
    return { checkpoints: [], lastSessionId: null };
  }
}

function writeStore(store: CheckpointStore): void {
  ensureDir();
  fs.writeFileSync(getStorePath(), JSON.stringify(store, null, 2), "utf-8");
}

// ---- Public API ----

/**
 * Save a checkpoint for a session.
 * Auto-prunes old checkpoints for the same session.
 */
export function saveCheckpoint(
  sessionId: string,
  messages: any[],
  options?: { iteration?: number },
): Checkpoint {
  ensureDir();

  const store = readStore();
  const now = new Date().toISOString();
  const iteration = options?.iteration || 0;

  const checkpoint: Checkpoint = {
    id: `ck_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
    sessionId,
    createdAt: now,
    iteration,
    totalTokens: JSON.stringify(messages).length,
    messageCount: messages.length,
    messages,
    summary: `Iteration ${iteration}, ${messages.length} messages`,
  };

  // Save checkpoint file
  const ckPath = path.resolve(getStorageDir(), `${checkpoint.id}.json`);
  fs.writeFileSync(ckPath, JSON.stringify(checkpoint, null, 2), "utf-8");

  // Update store index
  store.checkpoints.push(checkpoint);
  store.lastSessionId = sessionId;

  // Prune: keep only MAX_CHECKPOINTS_PER_SESSION per session
  const sessionCks = store.checkpoints.filter((c) => c.sessionId === sessionId);
  if (sessionCks.length > MAX_CHECKPOINTS_PER_SESSION) {
    const toRemove = sessionCks
      .sort((a, b) => new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime())
      .slice(0, sessionCks.length - MAX_CHECKPOINTS_PER_SESSION);

    for (const ck of toRemove) {
      try {
        fs.unlinkSync(path.resolve(getStorageDir(), `${ck.id}.json`));
      } catch { /* best-effort */ }
    }

    store.checkpoints = store.checkpoints.filter(
      (c) => !toRemove.find((r) => r.id === c.id),
    );
  }

  // Global size pruning: if total > 1GB, remove oldest checkpoints
  pruneBySize(store);

  writeStore(store);
  return checkpoint;
}

/**
 * Get the latest checkpoint for a session.
 */
export function getLatestCheckpoint(sessionId?: string): Checkpoint | null {
  const store = readStore();

  const targetSession = sessionId || store.lastSessionId;
  if (!targetSession) return null;

  const sessionCks = store.checkpoints
    .filter((c) => c.sessionId === targetSession)
    .sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());

  if (sessionCks.length === 0) return null;

  // Try to load the full checkpoint file
  const ckPath = path.resolve(getStorageDir(), `${sessionCks[0].id}.json`);
  try {
    return JSON.parse(fs.readFileSync(ckPath, "utf-8"));
  } catch {
    return sessionCks[0]; // Fall back to index entry
  }
}

/**
 * List all checkpoints.
 */
export function listCheckpoints(): { checkpoint: Checkpoint; hasFile: boolean }[] {
  const store = readStore();
  return store.checkpoints.map((ck) => ({
    checkpoint: ck,
    hasFile: fs.existsSync(path.resolve(getStorageDir(), `${ck.id}.json`)),
  }));
}

/**
 * Get the last session ID (for auto-resume on restart).
 */
export function getLastSessionId(): string | null {
  return readStore().lastSessionId;
}

/**
 * Delete checkpoints for a session.
 */
export function deleteSession(sessionId: string): boolean {
  const store = readStore();
  const toDelete = store.checkpoints.filter((c) => c.sessionId === sessionId);

  for (const ck of toDelete) {
    try {
      fs.unlinkSync(path.resolve(getStorageDir(), `${ck.id}.json`));
    } catch { /* best-effort */ }
  }

  store.checkpoints = store.checkpoints.filter((c) => c.sessionId !== sessionId);
  if (store.lastSessionId === sessionId) {
    store.lastSessionId = store.checkpoints.length > 0
      ? store.checkpoints[store.checkpoints.length - 1].sessionId
      : null;
  }

  writeStore(store);
  return toDelete.length > 0;
}

/**
 * Check if there are checkpoints to resume.
 */
export function hasResumableSession(): boolean {
  const store = readStore();
  if (!store.lastSessionId) return false;
  return store.checkpoints.some((c) => c.sessionId === store.lastSessionId);
}

/**
 * Get total storage size.
 */
export function getStorageStats(): { totalBytes: number; totalCheckpoints: number; sessions: number } {
  const store = readStore();
  let totalBytes = 0;
  for (const ck of store.checkpoints) {
    const ckPath = path.resolve(getStorageDir(), `${ck.id}.json`);
    try {
      totalBytes += fs.statSync(ckPath).size;
    } catch {
      totalBytes += 1024; // estimate
    }
  }
  return {
    totalBytes,
    totalCheckpoints: store.checkpoints.length,
    sessions: new Set(store.checkpoints.map((c) => c.sessionId)).size,
  };
}

// ---- Private ----

function pruneBySize(store: CheckpointStore): void {
  // Calculate actual size on disk
  let totalSize = 0;
  const ckSizes: { id: string; size: number }[] = [];

  for (const ck of store.checkpoints) {
    const ckPath = path.resolve(getStorageDir(), `${ck.id}.json`);
    try {
      const size = fs.statSync(ckPath).size;
      totalSize += size;
      ckSizes.push({ id: ck.id, size });
    } catch { /* deleted already */ }
  }

  if (totalSize <= MAX_TOTAL_STORAGE_BYTES) return;

  // Remove oldest checkpoints until under limit
  const sorted = [...store.checkpoints].sort(
    (a, b) => new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime(),
  );

  for (const ck of sorted) {
    if (totalSize <= MAX_TOTAL_STORAGE_BYTES) break;

    const ckPath = path.resolve(getStorageDir(), `${ck.id}.json`);
    try {
      const size = fs.statSync(ckPath).size;
      fs.unlinkSync(ckPath);
      totalSize -= size;
    } catch { /* already deleted */ }

    store.checkpoints = store.checkpoints.filter((c) => c.id !== ck.id);
  }
}
