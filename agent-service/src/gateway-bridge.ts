/**
 * Gateway Bridge — routes incoming platform messages to the Agent Loop
 */

import runAgent from "./agent-loop";
import { createExecutorTools, createMemoryTools, createSkillTools } from "./agent-tools";
import { SkillManager } from "./skill-system";

export class GatewayBridge {
  private executorUrl: string;
  private executorApiKey: string;
  private memoryHubUrl: string;
  private skillManager: SkillManager;

  constructor(config: { executorUrl: string; executorApiKey: string; memoryHubUrl: string }) {
    this.executorUrl = config.executorUrl;
    this.executorApiKey = config.executorApiKey;
    this.memoryHubUrl = config.memoryHubUrl;
    this.skillManager = new SkillManager();
  }

  async handleIncomingMessage(platform: string, channelId: string, userId: string, text: string): Promise<string> {
    const tools = [
      ...createExecutorTools(this.executorUrl, this.executorApiKey),
      ...createMemoryTools(this.memoryHubUrl),
      ...createSkillTools(),
    ];

    const matchingSkills = this.skillManager.findMatchingSkills(text);
    let enrichedPrompt = text;
    if (matchingSkills.length > 0) {
      enrichedPrompt = text + "\n\nRelevant skills:\n" +
        matchingSkills.slice(0, 3).map((s: any) => "- " + s.name + ": " + s.description).join("\n");
    }

    const result = await runAgent(enrichedPrompt, { tools, userId });

    if (result.success) {
      try {
        this.skillManager.generateSkill({
          workflowId: "chat_" + Date.now(), taskText: text,
          steps: ["Agent processed via " + platform], success: true, durationMs: 0,
          timestamp: new Date().toISOString(),
        });
      } catch { /* best-effort */ }
    }

    return result.success ? result.output : "Error: " + (result.error || "unknown");
  }
}
