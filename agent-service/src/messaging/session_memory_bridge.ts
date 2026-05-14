import axios from 'axios';
import { createClient, RedisClientType } from 'redis';

const MEMORY_HUB_URL = process.env.MEMORY_HUB_URL || 'http://localhost:8081';
const EXECUTOR_URL = process.env.EXECUTOR_URL || 'http://localhost:8080';

interface ConversationMemory {
  id: string;
  content: string;
  memory_type: string;
  importance: number;
  metadata: {
    platform: string;
    userId: string;
    user_input: string;
    agent_response: string;
  };
  created_at: number;
  last_access: number;
}

interface UserMapping {
  internalId: string;
  externalIds: Record<string, string>;
}

export class SessionMemoryBridge {
  private redis: RedisClientType | null = null;
  private userMappingCache: Map<string, UserMapping> = new Map();

  async init(): Promise<void> {
    const redisUrl = process.env.REDIS_URL;
    if (redisUrl) {
      this.redis = createClient({ url: redisUrl });
      await this.redis.connect();
      console.log('[SessionMemoryBridge] Redis connected');
    }
  }

  async storeConversation(
    platform: string,
    userId: string,
    userInput: string,
    agentResponse: string,
    workflowId?: string
  ): Promise<void> {
    const internalUserId = await this.resolveInternalUserId(platform, userId);

    const content = `User: ${userInput}\nAgent: ${agentResponse}`;
    const metadata = {
      platform,
      userId: internalUserId,
      user_input: userInput,
      agent_response: agentResponse,
      workflow_id: workflowId || null,
    };

    try {
      await axios.post(`${MEMORY_HUB_URL}/memories`, {
        content,
        type: 'ConversationContext',
        importance: 0.5,
        metadata,
      });
      console.log(`[SessionMemoryBridge] Stored conversation for user ${internalUserId}`);
    } catch (error: any) {
      console.error('[SessionMemoryBridge] Failed to store:', error.message);
    }
  }

  async retrieveUserContext(
    platform: string,
    userId: string,
    k: number = 3
  ): Promise<string> {
    const internalUserId = await this.resolveInternalUserId(platform, userId);

    try {
      const response = await axios.get(
        `${MEMORY_HUB_URL}/memories/search?q=${encodeURIComponent(internalUserId)}&k=${k}`
      );
      
      const memories = response.data.memories || [];
      if (memories.length === 0) return '';

      return memories
        .map((m: ConversationMemory) => m.content)
        .join('\n---\n');
    } catch (error: any) {
      console.error('[SessionMemoryBridge] Failed to retrieve:', error.message);
      return '';
    }
  }

  async mapUsers(externalId1: string, platform1: string, externalId2: string, platform2: string): Promise<string> {
    const key1 = `${platform1}:${externalId1}`;
    const key2 = `${platform2}:${externalId2}`;
    
    let internalId = `user_${Date.now()}`;

    if (this.userMappingCache.has(key1)) {
      internalId = this.userMappingCache.get(key1)!.internalId;
    } else if (this.userMappingCache.has(key2)) {
      internalId = this.userMappingCache.get(key2)!.internalId;
    }

    this.userMappingCache.set(key1, { internalId, externalIds: { [platform1]: externalId1 } });
    this.userMappingCache.set(key2, { internalId, externalIds: { [platform2]: externalId2 } });

    if (this.redis) {
      await this.redis.hSet('user_mappings', key1, internalId);
      await this.redis.hSet('user_mappings', key2, internalId);
    }

    console.log(`[SessionMemoryBridge] Mapped ${key1} <-> ${key2} -> ${internalId}`);
    return internalId;
  }

  private async resolveInternalUserId(platform: string, externalId: string): Promise<string> {
    const key = `${platform}:${externalId}`;

    if (this.userMappingCache.has(key)) {
      return this.userMappingCache.get(key)!.internalId;
    }

    if (this.redis) {
      const cached = await this.redis.hGet('user_mappings', key);
      if (cached) {
        this.userMappingCache.set(key, { internalId: cached, externalIds: { [platform]: externalId } });
        return cached;
      }
    }

    return externalId;
  }
}

export const sessionMemoryBridge = new SessionMemoryBridge();