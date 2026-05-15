/**
 * Goal Loop — Hermes Ralph loop / persistent goal tracking
 *
 * Allows the agent to maintain a persistent goal across sessions.
 * The goal is injected into the system prompt and periodically evaluated.
 */

import * as fs from "node:fs";
import * as path from "node:path";

export interface Goal {
  id: string;
  text: string;
  status: "active" | "completed" | "abandoned";
  progress: string;
  createdAt: string;
  updatedAt: string;
  checkCount: number;
}

const GOALS_DIR = path.resolve(
  process.env.MEMFLOW_RUNTIME_ROOT || path.resolve(process.cwd(), "..", ".memflow-runtime"),
  "goals",
);

function ensureDir(): void {
  if (!fs.existsSync(GOALS_DIR)) fs.mkdirSync(GOALS_DIR, { recursive: true });
}

function getGoalPath(id: string): string {
  return path.resolve(GOALS_DIR, `${id}.json`);
}

export function setGoal(text: string): Goal {
  ensureDir();
  const id = `goal_${Date.now()}`;
  const goal: Goal = {
    id,
    text,
    status: "active",
    progress: "Not started",
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    checkCount: 0,
  };
  // Remove old active goals
  for (const existing of listGoals()) {
    if (existing.status === "active") {
      fs.unlinkSync(getGoalPath(existing.id));
    }
  }
  fs.writeFileSync(getGoalPath(id), JSON.stringify(goal, null, 2), "utf-8");
  return goal;
}

export function getActiveGoal(): Goal | null {
  ensureDir();
  const goals = listGoals();
  return goals.find((g) => g.status === "active") || null;
}

export function getGoal(id: string): Goal | null {
  try {
    return JSON.parse(fs.readFileSync(getGoalPath(id), "utf-8")) as Goal;
  } catch {
    return null;
  }
}

export function listGoals(): Goal[] {
  ensureDir();
  try {
    return fs.readdirSync(GOALS_DIR)
      .filter((f) => f.endsWith(".json"))
      .map((f) => {
        try { return JSON.parse(fs.readFileSync(path.join(GOALS_DIR, f), "utf-8")) as Goal; }
        catch { return null; }
      })
      .filter(Boolean) as Goal[];
  } catch { return []; }
}

export function updateGoalProgress(progress: string): Goal | null {
  const goal = getActiveGoal();
  if (!goal) return null;
  goal.progress = progress;
  goal.updatedAt = new Date().toISOString();
  goal.checkCount++;
  fs.writeFileSync(getGoalPath(goal.id), JSON.stringify(goal, null, 2), "utf-8");
  return goal;
}

export function completeGoal(): Goal | null {
  const goal = getActiveGoal();
  if (!goal) return null;
  goal.status = "completed";
  goal.progress = "Completed";
  goal.updatedAt = new Date().toISOString();
  fs.writeFileSync(getGoalPath(goal.id), JSON.stringify(goal, null, 2), "utf-8");
  return goal;
}

/** Format the goal for system prompt injection */
export function formatGoalPrompt(): string {
  const goal = getActiveGoal();
  if (!goal) return "";
  return `\n[Active Goal]\nYou are working toward: ${goal.text}\nCurrent progress: ${goal.progress}\nDo not stop until the goal is complete.`;
}
