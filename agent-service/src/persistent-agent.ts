/**
 * Persistent Agent — connects AgentLoop → SkillManager → Memory
 * Uses the new runAgent() API (Hermes-aligned multi-provider agent loop)
 */

import runAgent from "./agent-loop";
import { SkillManager, type ExecutionRecord } from "./skill-system";
import { createExecutorTools, createMemoryTools, createSkillTools } from "./agent-tools";

export interface AgentIdentity {
  id: string; name: string; createdAt: string;
  totalTasks: number; successRate: number; learnedSkills: number;
}

export class PersistentAgent {
  private identity: AgentIdentity;
  private skillManager: SkillManager;
  private sessionHistory: ExecutionRecord[] = [];

  constructor(name?: string) {
    this.skillManager = new SkillManager();
    this.identity = {
      id: "agent_" + Date.now(), name: name || "MemFlow",
      createdAt: new Date().toISOString(), totalTasks: 0, successRate: 1.0,
      learnedSkills: this.skillManager.listSkills().length,
    };
  }

  async executeTask(text: string, executorUrl: string, executorKey: string, memoryUrl: string): Promise<string> {
    const tools = [
      ...createExecutorTools(executorUrl, executorKey),
      ...createMemoryTools(memoryUrl),
      ...createSkillTools(),
    ];
    const startTime = Date.now();
    const result = await runAgent(text, { tools });
    const duration = Date.now() - startTime;

    const record: ExecutionRecord = {
      workflowId: "agent_" + Date.now(), taskText: text,
      steps: result.output ? ["Agent processed"] : [],
      success: result.success, durationMs: duration,
      timestamp: new Date().toISOString(),
    };
    this.sessionHistory.push(record);
    this.identity.totalTasks++;

    if (result.success && text.split(/\s+/).length > 5) {
      this.skillManager.generateSkill(record);
      this.identity.learnedSkills = this.skillManager.listSkills().length;
    }
    const successes = this.sessionHistory.filter((r) => r.success).length;
    this.identity.successRate = successes / this.sessionHistory.length;
    return result.output || "Failed: " + (result.error || "unknown");
  }

  getIdentity(): AgentIdentity { return this.identity; }
  getHistory(): ExecutionRecord[] { return this.sessionHistory; }
}
