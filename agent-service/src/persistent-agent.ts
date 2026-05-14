/**
 * T3.6: Persistent Agent + Complete Learning Loop
 * 
 * Connects AgentLoop → SkillManager → Memory → Improvement Cycle
 * Agents persist across sessions and improve over time.
 */

import { AgentLoop } from "./agent-loop";
import { SkillManager, type Skill, type ExecutionRecord } from "./skill-system";
import { createExecutorTools, createMemoryTools, createSkillTools } from "./agent-tools";
import { getLLMSettings } from "./llm_settings";

export interface AgentIdentity {
  id: string;
  name: string;
  createdAt: string;
  totalTasks: number;
  successRate: number;
  learnedSkills: number;
}

export class PersistentAgent {
  private identity: AgentIdentity;
  private skillManager: SkillManager;
  private sessionHistory: ExecutionRecord[] = [];

  constructor(name?: string) {
    this.skillManager = new SkillManager();
    this.identity = {
      id: `agent_${Date.now()}`,
      name: name || "MemFlow",
      createdAt: new Date().toISOString(),
      totalTasks: 0,
      successRate: 1.0,
      learnedSkills: this.skillManager.listSkills().length,
    };
  }

  async executeTask(
    text: string,
    executorUrl: string,
    executorKey: string,
    memoryUrl: string,
  ): Promise<string> {
    const loop = new AgentLoop({
      systemPrompt: this.buildSystemPrompt(),
    });

    // Register tools
    const allTools = [
      ...createExecutorTools(executorUrl, executorKey),
      ...createMemoryTools(memoryUrl),
      ...createSkillTools(),
    ];
    for (const t of allTools) loop.addTool(t);

    const startTime = Date.now();
    const result = await loop.run(text);
    const duration = Date.now() - startTime;

    // Record execution for learning
    const record: ExecutionRecord = {
      workflowId: `agent_${Date.now()}`,
      taskText: text,
      steps: result.output ? ["Agent processed"] : [],
      success: result.success,
      durationMs: duration,
      timestamp: new Date().toISOString(),
    };
    this.sessionHistory.push(record);
    this.identity.totalTasks++;

    // Auto-learn: generate skill from successful execution
    if (result.success && text.split(/\s+/).length > 5) {
      this.skillManager.generateSkill(record);
      this.identity.learnedSkills = this.skillManager.listSkills().length;
    }

    // Update success rate
    const successes = this.sessionHistory.filter((r) => r.success).length;
    this.identity.successRate = successes / this.sessionHistory.length;

    return result.success
      ? result.output
      : `Failed after ${result.iterations} iterations: ${result.error}`;
  }

  private buildSystemPrompt(): string {
    const skills = this.skillManager.listSkills();
    const skillSection = skills.length > 0
      ? `\n\nAvailable skills you have learned:\n${skills.map((s) => `- ${s.name}: ${s.description}`).join("\n")}\n\nUse these skills when relevant tasks are requested.`
      : "";

    return `You are ${this.identity.name}, a persistent AI agent that improves over time.
You have completed ${this.identity.totalTasks} tasks with a ${(this.identity.successRate * 100).toFixed(0)}% success rate.
You have learned ${this.identity.learnedSkills} reusable skills.
Be efficient, use tools appropriately, and learn from each interaction.${skillSection}`;
  }

  getIdentity(): AgentIdentity { return this.identity; }
  getHistory(): ExecutionRecord[] { return this.sessionHistory; }
}
