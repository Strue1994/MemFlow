/**
 * T1.2: Message Gateway Integration
 * 
 * Connects the Rust gateway (Platform trait + GatewayRouter) to the TypeScript
 * agent loop. Routes incoming messages from Telegram/Discord/etc. to the agent.
 */

import { createAgentLoop } from './agent-loop';
import { createExecutorTools, createMemoryTools, createSkillTools } from './agent-tools';
import { SkillManager } from './skill-system';

export interface GatewayConfig {
  executorUrl: string;
  executorApiKey: string;
  memoryHubUrl: string;
}

export class GatewayBridge {
  private config: GatewayConfig;
  private skillManager: SkillManager;

  constructor(config: GatewayConfig) {
    this.config = config;
    this.skillManager = new SkillManager();
  }

  /**
   * Process an incoming message from any platform (Telegram, Discord, Slack, etc.)
   * Returns the agent"s response text.
   */
  async handleIncomingMessage(
    platform: string,
    channelId: string,
    userId: string,
    text: string,
  ): Promise<string> {
    const loop = await createAgentLoop();

    // Register tools
    const executorTools = createExecutorTools(this.config.executorUrl, this.config.executorApiKey);
    const memoryTools = createMemoryTools(this.config.memoryHubUrl);
    const skillTools = createSkillTools();

    for (const tool of [...executorTools, ...memoryTools, ...skillTools]) {
      loop.addTool(tool);
    }

    // Check for matching skills first
    const matchingSkills = this.skillManager.findMatchingSkills(text);
    let enrichedPrompt = text;
    if (matchingSkills.length > 0) {
      const skillHint = matchingSkills
        .slice(0, 3)
        .map((s) => `- ${s.name}: ${s.description}`)
        .join('\n');
      enrichedPrompt = `${text}\n\nRelevant skills available:\n${skillHint}`;
    }

    const result = await loop.run(enrichedPrompt);

    if (result.success) {
      // Record successful execution for skill learning
      this.skillManager.generateSkill({
        workflowId: `chat_${Date.now()}`,
        taskText: text,
        steps: [`Agent processed via ${platform}`],
        success: true,
        durationMs: 0,
        timestamp: new Date().toISOString(),
      });
    }

    return result.success
      ? result.output
      : `I encountered an error: ${result.error || "Unknown error"}`;
  }
}

export default GatewayBridge;
