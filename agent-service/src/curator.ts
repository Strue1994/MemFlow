/**
 * Curator — Hermes-aligned self-learning loop for MemFlow
 *
 * Features:
 * - Monitors agent execution events and extracts patterns
 * - Auto-generates skills from successful executions
 * - Periodic review cycle: grade, merge, prune
 * - Bridges to learning-engine (Rust) via HTTP
 * - Reports via /curator/report
 */

import { SkillManager, type Skill, type ExecutionRecord } from "./skill-system";

// ---- Types ----

export interface CuratorReport {
  cycleId: string;
  startedAt: string;
  durationMs: number;
  newSkills: number;
  mergedSkills: number;
  prunedSkills: number;
  totalSkills: number;
  topSkills: { name: string; usage: number }[];
  summary: string;
}

export interface CuratorStatus {
  lastRun: string | null;
  nextRun: string | null;
  totalRecords: number;
  totalSkills: number;
  skillsByCategory: Record<string, number>;
}

// ---- Curator Engine ----

const SKILLS_RETENTION_DAYS = 30;
const MIN_EXECUTIONS_FOR_SKILL = 2;
const SIMILARITY_THRESHOLD = 0.8; // cosine similarity threshold for merging

export class Curator {
  private skillManager: SkillManager;
  private executionHistory: ExecutionRecord[] = [];
  private cycleCount = 0;

  constructor(skillManager?: SkillManager) {
    this.skillManager = skillManager || new SkillManager();
  }

  /** Record an agent execution for later analysis */
  recordExecution(record: ExecutionRecord): void {
    this.executionHistory.push(record);
    // Cap history at 1000 entries
    if (this.executionHistory.length > 1000) {
      this.executionHistory = this.executionHistory.slice(-500);
    }
  }

  /** Get all recorded executions */
  getExecutionHistory(): ExecutionRecord[] {
    return [...this.executionHistory];
  }

  /** Run one curator cycle */
  async runCycle(): Promise<CuratorReport> {
    const startTime = Date.now();
    this.cycleCount++;
    const cycleId = `curator_${Date.now()}`;

    let newSkills = 0;
    let mergedSkills = 0;
    let prunedSkills = 0;

    // Phase 1: Generate new skills from successful executions
    const generated = this.generateSkillsFromHistory();
    newSkills += generated;

    // Phase 2: Merge similar skills
    mergedSkills = this.mergeSimilarSkills();

    // Phase 3: Prune old/unused skills
    prunedSkills = this.pruneOldSkills();

    // Phase 4: Get top skills by usage
    const allSkills = this.skillManager.listSkills();
    const topSkills = allSkills
      .sort((a, b) => b.execution_count - a.execution_count)
      .slice(0, 5)
      .map((s) => ({ name: s.name, usage: s.execution_count }));

    const durationMs = Date.now() - startTime;

    return {
      cycleId,
      startedAt: new Date(startTime).toISOString(),
      durationMs,
      newSkills,
      mergedSkills,
      prunedSkills,
      totalSkills: allSkills.length,
      topSkills,
      summary: this.generateSummary(newSkills, mergedSkills, prunedSkills, allSkills.length),
    };
  }

  /** Get current curator status */
  getStatus(): CuratorStatus {
    const allSkills = this.skillManager.listSkills();
    const skillsByCategory: Record<string, number> = {};
    for (const s of allSkills) {
      skillsByCategory[s.category] = (skillsByCategory[s.category] || 0) + 1;
    }

    return {
      lastRun: this.cycleCount > 0 ? new Date().toISOString() : null,
      nextRun: null,
      totalRecords: this.executionHistory.length,
      totalSkills: allSkills.length,
      skillsByCategory,
    };
  }

  // ---- Private: Skill Generation ----

  private generateSkillsFromHistory(): number {
    const existingSkills = this.skillManager.listSkills();
    const existingNames = new Set(existingSkills.map((s) => s.name.toLowerCase()));

    // Group executions by task type
    const taskGroups = new Map<string, ExecutionRecord[]>();
    for (const record of this.executionHistory) {
      // Extract key terms from task text (first 3 meaningful words)
      const key = record.taskText
        .toLowerCase()
        .replace(/[^a-z0-9\s]/g, "")
        .split(/\s+/)
        .filter((w) => w.length > 3)
        .slice(0, 3)
        .join("_");
      if (!key) continue;

      if (!taskGroups.has(key)) taskGroups.set(key, []);
      taskGroups.get(key)!.push(record);
    }

    let count = 0;
    for (const [, records] of taskGroups) {
      if (records.length < MIN_EXECUTIONS_FOR_SKILL) continue;

      const successful = records.filter((r) => r.success);
      if (successful.length === 0) continue;

      const taskText = successful[0].taskText;
      const name = this.makeSkillName(taskText);
      if (existingNames.has(name.toLowerCase())) continue;

      // Calculate success rate
      const successRate = successful.length / records.length;
      if (successRate < 0.5) continue; // Skip if most executions failed

      // Create skill from execution pattern
      const skill: Skill = {
        id: `learned_${Date.now()}_${count}`,
        name,
        description: `Learned from ${successful.length} execution(s): ${taskText.slice(0, 100)}`,
        category: "learned",
        keywords: taskText
          .toLowerCase()
          .replace(/[^a-z0-9\s]/g, "")
          .split(/\s+/)
          .filter((w) => w.length > 3)
          .slice(0, 8),
        pattern: successful[0]?.steps.join(" → ") || "",
        steps: successful[0]?.steps.map((step, i) => ({
          order: i + 1,
          action: step,
          description: `Step ${i + 1}: ${step}`,
        })) || [],
        examples: successful.slice(0, 3).map((r) => ({
          input: r.taskText,
          output: `Completed in ${r.durationMs}ms`,
          description: r.taskText,
        })),
        version: "1.0.0",
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        execution_count: successful.length,
        success_rate: successRate,
      };

      this.skillManager.saveSkill(skill);
      count++;
    }

    return count;
  }

  // ---- Private: Merge Similar Skills ----

  private mergeSimilarSkills(): number {
    const skills = this.skillManager.listSkills().filter((s) => s.category === "learned");
    let merged = 0;

    for (let i = 0; i < skills.length; i++) {
      for (let j = i + 1; j < skills.length; j++) {
        const similarity = this.calculateSimilarity(skills[i], skills[j]);
        if (similarity > SIMILARITY_THRESHOLD) {
          // Merge j into i
          skills[i].execution_count += skills[j].execution_count;
          skills[i].examples.push(...skills[j].examples);
          skills[i].keywords = [...new Set([...skills[i].keywords, ...skills[j].keywords])];
          skills[i].name = `${skills[i].name} / ${skills[j].name}`;
          skills[i].description = `Merged: ${skills[i].description} | ${skills[j].description}`;

          // Remove j by overwriting with placeholder
          this.skillManager.saveSkill({ ...skills[j], id: `merged_${Date.now()}_${j}`, name: `__merged_${skills[j].name}` });
          merged++;
        }
      }
    }

    return merged;
  }

  // ---- Private: Prune Old Skills ----

  private pruneOldSkills(): number {
    const skills = this.skillManager.listSkills();
    const cutoff = new Date();
    cutoff.setDate(cutoff.getDate() - SKILLS_RETENTION_DAYS);

    let pruned = 0;
    for (const skill of skills) {
      if (skill.category !== "learned") continue;

      const updatedAt = new Date(skill.updated_at);
      if (updatedAt < cutoff && skill.execution_count < 3) {
        // Mark for pruning — don't delete, just rename to archive
        this.skillManager.saveSkill({
          ...skill,
          id: `archived_${skill.id}`,
          name: `[ARCHIVED] ${skill.name}`,
          category: "archived",
        });
        pruned++;
      }
    }

    return pruned;
  }

  // ---- Private: Helpers ----

  private makeSkillName(text: string): string {
    const cleaned = text.replace(/[^a-zA-Z0-9\s]/g, "").trim();
    const words = cleaned.split(/\s+/).slice(0, 4);
    return words.map((w) => w[0].toUpperCase() + w.slice(1)).join(" ");
  }

  private calculateSimilarity(a: Skill, b: Skill): number {
    const keywords = new Set([...a.keywords, ...b.keywords]);
    if (keywords.size === 0) return 0;

    let matches = 0;
    for (const kw of a.keywords) {
      if (b.keywords.includes(kw)) matches++;
    }

    return matches / Math.max(a.keywords.length, b.keywords.length, 1);
  }

  private generateSummary(newS: number, merged: number, pruned: number, total: number): string {
    const parts: string[] = [];
    if (newS > 0) parts.push(`${newS} new skill(s) created from execution patterns`);
    if (merged > 0) parts.push(`${merged} similar skill(s) merged`);
    if (pruned > 0) parts.push(`${pruned} unused skill(s) archived`);
    if (newS === 0 && merged === 0 && pruned === 0) parts.push("No changes needed");
    parts.push(`Total skill library: ${total}`);
    return parts.join(". ");
  }
}

// ---- Global curator instance ----

export const globalCurator = new Curator();
