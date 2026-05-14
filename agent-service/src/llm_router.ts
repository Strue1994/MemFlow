import { createClient, RedisClientType } from 'redis';
import crypto from 'crypto';

export interface LLMConfig {
  provider: 'openai' | 'anthropic' | 'gemini' | 'local';
  model: string;
  apiKey?: string;
  endpoint?: string;
  maxTokens: number;
  temperature: number;
  timeoutMs: number;
  costPer1kInput: number;
  costPer1kOutput: number;
}

export interface TaskComplexity {
  level: 'simple' | 'medium' | 'complex';
  estimatedTokens: number;
  requiredCapabilities: string[];
}

export interface LLMRequest {
  taskType: string;
  prompt: string;
  maxBudget?: number;
  maxLatency?: number;
  context?: Record<string, unknown>;
}

export interface LLMResponse {
  content: string;
  model: string;
  tokensUsed: { input: number; output: number };
  cost: number;
  latencyMs: number;
  cached: boolean;
}

export interface CostRecord {
  timestamp: number;
  model: string;
  taskType: string;
  tokensIn: number;
  tokensOut: number;
  cost: number;
  userId?: string;
}

export const DEFAULT_LLM_CONFIGS: Record<string, LLMConfig> = {
  gpt4o: {
    provider: 'openai',
    model: 'gpt-4o',
    maxTokens: 4096,
    temperature: 0.7,
    timeoutMs: 30000,
    costPer1kInput: 0.005,
    costPer1kOutput: 0.015,
  },
  gpt35: {
    provider: 'openai',
    model: 'gpt-3.5-turbo',
    maxTokens: 4096,
    temperature: 0.7,
    timeoutMs: 15000,
    costPer1kInput: 0.001,
    costPer1kOutput: 0.002,
  },
  claude3opus: {
    provider: 'anthropic',
    model: 'claude-3-opus-20240229',
    maxTokens: 4096,
    temperature: 0.7,
    timeoutMs: 30000,
    costPer1kInput: 0.015,
    costPer1kOutput: 0.075,
  },
  claude3haiku: {
    provider: 'anthropic',
    model: 'claude-3-haiku-20240307',
    maxTokens: 4096,
    temperature: 0.7,
    timeoutMs: 10000,
    costPer1kInput: 0.00025,
    costPer1kOutput: 0.00125,
  },
  local7b: {
    provider: 'local',
    model: 'llama-7b',
    endpoint: 'http://localhost:11434/api/generate',
    maxTokens: 2048,
    temperature: 0.7,
    timeoutMs: 60000,
    costPer1kInput: 0,
    costPer1kOutput: 0,
  },
};

const COMPLEXITY_PATTERNS = {
  simple: [
    /pattern match/i,
    /expression/i,
    /simple/i,
    /basic/i,
    /extract\s+\w+/i,
    /format/i,
  ],
  medium: [
    /generate/i,
    /create.*workflow/i,
    /single.*step/i,
    /transform/i,
    /convert/i,
  ],
  complex: [
    /multi.*node/i,
    /conditional/i,
    /branch/i,
    /error.*recovery/i,
    /complex/i,
    /optimize/i,
    /refactor/i,
  ],
};

export class LLMRouter {
  private redisClient: RedisClientType | null = null;
  private configs: Map<string, LLMConfig> = new Map();
  private costRecords: CostRecord[] = [];
  private useCache: boolean = true;
  private cacheTTL: number = 86400;

  constructor(configs?: Partial<Record<string, LLMConfig>>, redisUrl?: string) {
    if (configs) {
      Object.entries(configs).forEach(([key, config]) => {
        this.configs.set(key, { ...DEFAULT_LLM_CONFIGS[key], ...config });
      });
    } else {
      Object.entries(DEFAULT_LLM_CONFIGS).forEach(([key, config]) => {
        this.configs.set(key, config);
      });
    }

    if (redisUrl) {
      this.initRedis(redisUrl);
    }
  }

  private async initRedis(url: string) {
    try {
      this.redisClient = createClient({ url });
      await this.redisClient.connect();
    } catch (error) {
      console.warn('Redis connection failed, caching disabled:', error);
      this.useCache = false;
    }
  }

  analyzeComplexity(taskType: string, prompt: string): TaskComplexity {
    const text = `${taskType} ${prompt}`;
    
    let matchedComplex = 0;
    let matchedMedium = 0;
    let matchedSimple = 0;

    COMPLEXITY_PATTERNS.complex.forEach(p => {
      if (p.test(text)) matchedComplex++;
    });
    COMPLEXITY_PATTERNS.medium.forEach(p => {
      if (p.test(text)) matchedMedium++;
    });
    COMPLEXITY_PATTERNS.simple.forEach(p => {
      if (p.test(text)) matchedSimple++;
    });

    const estimatedTokens = Math.ceil(prompt.length / 4);

    if (matchedComplex > 0 || estimatedTokens > 2000) {
      return { level: 'complex', estimatedTokens, requiredCapabilities: ['reasoning', 'code'] };
    } else if (matchedMedium > 0 || estimatedTokens > 500) {
      return { level: 'medium', estimatedTokens, requiredCapabilities: ['generation'] };
    } else {
      return { level: 'simple', estimatedTokens, requiredCapabilities: ['extraction'] };
    }
  }

  selectModel(
    complexity: TaskComplexity,
    maxBudget?: number,
    maxLatency?: number
  ): string {
    const candidates: string[] = [];

    if (complexity.level === 'simple') {
      candidates.push('local7b', 'claude3haiku', 'gpt35');
    } else if (complexity.level === 'medium') {
      candidates.push('claude3haiku', 'gpt35', 'gpt4o', 'claude3opus');
    } else {
      candidates.push('gpt4o', 'claude3opus', 'claude3haiku', 'gpt35');
    }

    for (const key of candidates) {
      const config = this.configs.get(key);
      if (!config) continue;

      if (maxBudget && config.costPer1kInput > maxBudget) continue;
      if (maxLatency && config.timeoutMs > maxLatency) continue;

      return key;
    }

    return 'gpt35';
  }

  private computeCacheKey(taskType: string, prompt: string): string {
    const hash = crypto.createHash('sha256').update(`${taskType}:${prompt}`).digest('hex');
    return `llm_cache:${taskType}:${hash.slice(0, 16)}`;
  }

  async getCachedResult(cacheKey: string): Promise<string | null> {
    if (!this.useCache || !this.redisClient) return null;
    try {
      return await this.redisClient.get(cacheKey);
    } catch {
      return null;
    }
  }

  async cacheResult(cacheKey: string, content: string): Promise<void> {
    if (!this.useCache || !this.redisClient) return;
    try {
      await this.redisClient.setEx(cacheKey, this.cacheTTL, content);
    } catch {
      // Silent fail
    }
  }

  async route(request: LLMRequest): Promise<LLMResponse> {
    const complexity = this.analyzeComplexity(request.taskType, request.prompt);
    const primaryModelKey = this.selectModel(complexity, request.maxBudget, request.maxLatency);
    const config = this.configs.get(primaryModelKey)!;

    const cacheKey = this.computeCacheKey(request.taskType, request.prompt);
    const cachedContent = await this.getCachedResult(cacheKey);

    if (cachedContent) {
      return {
        content: cachedContent,
        model: config.model,
        tokensUsed: { input: 0, output: 0 },
        cost: 0,
        latencyMs: 0,
        cached: true,
      };
    }

    const startTime = Date.now();
    let response: LLMResponse | null = null;
    let lastError: Error | null = null;

    const fallbackOrder = this.getFallbackOrder(primaryModelKey);

    for (const modelKey of fallbackOrder) {
      try {
        const cfg = this.configs.get(modelKey)!;
        response = await this.callModel(cfg, request.prompt);
        break;
      } catch (error) {
        lastError = error as Error;
        console.warn(`Model ${modelKey} failed, trying fallback:`, error);
      }
    }

    if (!response) {
      throw lastError || new Error('All models failed');
    }

    await this.cacheResult(cacheKey, response.content);

    this.recordCost({
      timestamp: Date.now(),
      model: response.model,
      taskType: request.taskType,
      tokensIn: response.tokensUsed.input,
      tokensOut: response.tokensUsed.output,
      cost: response.cost,
    });

    return response;
  }

  private getFallbackOrder(primary: string): string[] {
    const fallbacks: Record<string, string[]> = {
      gpt4o: ['gpt4o', 'claude3opus', 'claude3haiku', 'gpt35'],
      gpt35: ['gpt35', 'claude3haiku'],
      claude3opus: ['claude3opus', 'claude3haiku', 'gpt4o'],
      claude3haiku: ['claude3haiku', 'gpt35'],
      local7b: ['local7b', 'gpt35'],
    };
    return fallbacks[primary] || [primary];
  }

  private async callModel(config: LLMConfig, prompt: string): Promise<LLMResponse> {
    const startTime = Date.now();

    let content: string;
    let tokensIn: number;
    let tokensOut: number;

    if (config.provider === 'openai') {
      const result = await this.callOpenAI(config, prompt);
      content = result.content;
      tokensIn = result.tokensIn;
      tokensOut = result.tokensOut;
    } else if (config.provider === 'anthropic') {
      const result = await this.callAnthropic(config, prompt);
      content = result.content;
      tokensIn = result.tokensIn;
      tokensOut = result.tokensOut;
    } else if (config.provider === 'local') {
      const result = await this.callLocal(config, prompt);
      content = result.content;
      tokensIn = Math.ceil(prompt.length / 4);
      tokensOut = Math.ceil(content.length / 4);
    } else {
      throw new Error(`Unknown provider: ${config.provider}`);
    }

    const latencyMs = Date.now() - startTime;
    const cost = (tokensIn / 1000) * config.costPer1kInput + (tokensOut / 1000) * config.costPer1kOutput;

    return {
      content,
      model: config.model,
      tokensUsed: { input: tokensIn, output: tokensOut },
      cost,
      latencyMs,
      cached: false,
    };
  }

  private async callOpenAI(config: LLMConfig, prompt: string) {
    const response = await fetch('https://api.openai.com/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${config.apiKey || process.env.OPENAI_API_KEY}`,
      },
      signal: AbortSignal.timeout(config.timeoutMs),
      body: JSON.stringify({
        model: config.model,
        messages: [{ role: 'user', content: prompt }],
        max_tokens: config.maxTokens,
        temperature: config.temperature,
      }),
    });

    if (!response.ok) {
      throw new Error(`OpenAI API error: ${response.status}`);
    }

    const data = await response.json() as { choices: { message: { content: string } }[]; usage: { prompt_tokens: number; completion_tokens: number } };
    return {
      content: data.choices[0]?.message?.content || '',
      tokensIn: data.usage?.prompt_tokens || 0,
      tokensOut: data.usage?.completion_tokens || 0,
    };
  }

  private async callAnthropic(config: LLMConfig, prompt: string) {
    const response = await fetch('https://api.anthropic.com/v1/messages', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-api-key': config.apiKey || process.env.ANTHROPIC_API_KEY,
        'anthropic-version': '2023-06-01',
      },
      signal: AbortSignal.timeout(config.timeoutMs),
      body: JSON.stringify({
        model: config.model,
        max_tokens: config.maxTokens,
        messages: [{ role: 'user', content: prompt }],
      }),
    });

    if (!response.ok) {
      throw new Error(`Anthropic API error: ${response.status}`);
    }

    const data = await response.json() as { content: { text: string }[]; usage: { input_tokens: number; output_tokens: number } };
    return {
      content: data.content[0]?.text || '',
      tokensIn: data.usage?.input_tokens || 0,
      tokensOut: data.usage?.output_tokens || 0,
    };
  }

  private async callLocal(config: LLMConfig, prompt: string) {
    const response = await fetch(config.endpoint!, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      signal: AbortSignal.timeout(config.timeoutMs),
      body: JSON.stringify({
        model: config.model,
        prompt,
        options: { temperature: config.temperature, num_predict: config.maxTokens },
      }),
    });

    if (!response.ok) {
      throw new Error(`Local model error: ${response.status}`);
    }

    const data = await response.json() as { response?: string };
    return {
      content: data.response || '',
      tokensIn: 0,
      tokensOut: 0,
    };
  }

  private recordCost(record: CostRecord): void {
    this.costRecords.push(record);
    if (this.costRecords.length > 10000) {
      this.costRecords = this.costRecords.slice(-5000);
    }
  }

  getCostBreakdown(period: 'daily' | 'weekly' = 'daily'): { date: string; totalCost: number; byModel: Record<string, number> }[] {
    const now = Date.now();
    const periodMs = period === 'daily' ? 86400000 : 604800000;
    const startTime = now - periodMs * 7;

    const filtered = this.costRecords.filter(r => r.timestamp >= startTime);

    const byDate: Record<string, { total: number; byModel: Record<string, number> }> = {};

    for (const record of filtered) {
      const date = new Date(record.timestamp).toISOString().split('T')[0];
      if (!byDate[date]) {
        byDate[date] = { total: 0, byModel: {} };
      }
      byDate[date].total += record.cost;
      byDate[date].byModel[record.model] = (byDate[date].byModel[record.model] || 0) + record.cost;
    }

    return Object.entries(byDate)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([date, data]) => ({ date, totalCost: data.total, byModel: data.byModel }));
  }

  async getCacheStats(): Promise<{ hitRate: number; size: number }> {
    if (!this.redisClient) return { hitRate: 0, size: 0 };
    try {
      const keys = await this.redisClient.keys('llm_cache:*');
      return { hitRate: 0, size: keys.length };
    } catch {
      return { hitRate: 0, size: 0 };
    }
  }

  async close(): Promise<void> {
    if (this.redisClient) {
      await this.redisClient.quit();
    }
  }
}

export function createLLMRouter(configs?: Partial<Record<string, LLMConfig>>, redisUrl?: string): LLMRouter {
  return new LLMRouter(configs, redisUrl);
}