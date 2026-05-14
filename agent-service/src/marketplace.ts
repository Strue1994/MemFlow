import { createClient, RedisClientType } from 'redis';

export interface MarketplaceWorkflow {
  id: string;
  remoteId: string;
  name: string;
  description: string;
  tags: string[];
  authorId: string;
  authorName: string;
  workflowJson: string;
  price?: number;
  rating: number;
  ratingCount: number;
  downloadCount: number;
  createdAt: number;
  updatedAt: number;
  verified: boolean;
  memflowVersion?: string;
}

export interface WorkflowVersion {
  version: number;
  changelog: string;
  createdAt: number;
  authorId: string;
  authorName: string;
  diff?: string;
}

export const CURRENT_MEMFLOW_VERSION = '1.0.0';

export interface CompatibilityResult {
  compatible: boolean;
  errors: string[];
  warnings: string[];
}

export function checkVersionCompatibility(workflow: MarketplaceWorkflow): CompatibilityResult {
  const errors: string[] = [];
  const warnings: string[] = [];
  
  if (workflow.memflowVersion) {
    const required = workflow.memflowVersion;
    const current = CURRENT_MEMFLOW_VERSION;
    
    if (required !== current) {
      errors.push(
        `Workflow requires memflow ${required}, current is ${current}`
      );
    }
  }
  
  return { compatible: errors.length === 0, errors, warnings };
}

export interface WorkflowRating {
  id: string;
  workflowId: string;
  userId: string;
  rating: number;
  comment?: string;
  createdAt: number;
  moderated: boolean;
}

export interface PublishRequest {
  workflowId: string;
  name: string;
  description: string;
  tags: string[];
  price?: number;
}

export interface SearchQuery {
  q?: string;
  tags?: string[];
  minRating?: number;
  sortBy?: 'rating' | 'downloads' | 'newest';
  limit?: number;
  offset?: number;
}

const SENSITIVE_PATTERNS = [
  /api[_-]?key/i,
  /password/i,
  /secret/i,
  /token/i,
  /bearer/i,
  /authorization/i,
  /x-api-key/i,
  /private[_-]?key/i,
];

const SENSITIVE_KEYS = [
  'apiKey',
  'api_key',
  'password',
  'secret',
  'token',
  'bearer',
  'authorization',
  'privateKey',
  'private_key',
];

function sanitizeWorkflow(json: string): string {
  let result = json;
  
  SENSITIVE_PATTERNS.forEach(pattern => {
    result = result.replace(pattern, '[REDACTED]');
  });

  try {
    const obj = JSON.parse(result);
    sanitizeObject(obj);
    return JSON.stringify(obj, null, 2);
  } catch {
    return result;
  }
}

function sanitizeObject(obj: Record<string, unknown>): void {
  for (const key of Object.keys(obj)) {
    if (SENSITIVE_KEYS.some(k => key.toLowerCase().includes(k.toLowerCase()))) {
      obj[key] = '[REDACTED]';
    } else if (typeof obj[key] === 'object' && obj[key] !== null) {
      sanitizeObject(obj[key] as Record<string, unknown>);
    }
  }
}

export class WorkflowMarketplace {
  private redis: RedisClientType | null = null;
  private workflows: Map<string, MarketplaceWorkflow> = new Map();
  private ratings: Map<string, WorkflowRating[]> = new Map();
  private initialized: boolean = false;

  constructor(redisUrl?: string) {
    if (redisUrl) {
      this.initRedis(redisUrl);
    }
  }

  private async initRedis(url: string) {
    try {
      this.redis = createClient({ url });
      await this.redis.connect();
      await this.loadFromRedis();
      this.initialized = true;
    } catch (error) {
      console.warn('Redis unavailable, using in-memory storage:', error);
    }
  }

  private async loadFromRedis(): Promise<void> {
    if (!this.redis) return;
    try {
      const keys = await this.redis.keys('marketplace:workflow:*');
      for (const key of keys) {
        const data = await this.redis.get(key);
        if (data) {
          const wf = JSON.parse(data) as MarketplaceWorkflow;
          this.workflows.set(wf.remoteId, wf);
        }
      }
    } catch (error) {
      console.error('Failed to load marketplace data:', error);
    }
  }

  private async saveToRedis(workflow: MarketplaceWorkflow): Promise<void> {
    if (!this.redis) return;
    try {
      await this.redis.setEx(
        `marketplace:workflow:${workflow.remoteId}`,
        86400 * 30,
        JSON.stringify(workflow)
      );
    } catch (error) {
      console.error('Failed to save workflow:', error);
    }
  }

  async publish(request: PublishRequest, userId: string, userName: string): Promise<MarketplaceWorkflow> {
    const existing = this.workflows.get(request.workflowId);
    if (existing) {
      throw new Error('Workflow already published');
    }

    const workflowJson = this.getWorkflowJson(request.workflowId);
    const sanitized = sanitizeWorkflow(workflowJson);

    const workflow: MarketplaceWorkflow = {
      id: `wf_${Date.now()}`,
      remoteId: request.workflowId,
      name: request.name,
      description: request.description,
      tags: request.tags,
      authorId: userId,
      authorName: userName,
      workflowJson: sanitized,
      price: request.price,
      rating: 0,
      ratingCount: 0,
      downloadCount: 0,
      createdAt: Date.now(),
      updatedAt: Date.now(),
      verified: false,
    };

    this.workflows.set(workflow.remoteId, workflow);
    await this.saveToRedis(workflow);

    return workflow;
  }

  private getWorkflowJson(workflowId: string): string {
    return JSON.stringify({
      name: `Workflow ${workflowId}`,
      nodes: [],
      connections: {},
    });
  }

  async search(query: SearchQuery): Promise<{ workflows: MarketplaceWorkflow[]; total: number }> {
    let results = Array.from(this.workflows.values());

    if (query.q) {
      const q = query.q.toLowerCase();
      results = results.filter(wf =>
        wf.name.toLowerCase().includes(q) ||
        wf.description.toLowerCase().includes(q) ||
        wf.tags.some(t => t.toLowerCase().includes(q))
      );
    }

    if (query.tags && query.tags.length > 0) {
      results = results.filter(wf =>
        query.tags!.some(t => wf.tags.includes(t))
      );
    }

    if (query.minRating) {
      results = results.filter(wf => wf.rating >= query.minRating!);
    }

    const total = results.length;

    switch (query.sortBy) {
      case 'rating':
        results.sort((a, b) => b.rating - a.rating);
        break;
      case 'downloads':
        results.sort((a, b) => b.downloadCount - a.downloadCount);
        break;
      case 'newest':
      default:
        results.sort((a, b) => b.createdAt - a.createdAt);
        break;
    }

    const offset = query.offset || 0;
    const limit = query.limit || 20;
    results = results.slice(offset, offset + limit);

    return { workflows: results, total };
  }

  async getById(remoteId: string): Promise<MarketplaceWorkflow | null> {
    return this.workflows.get(remoteId) || null;
  }

  async importWorkflow(remoteId: string, targetUserId: string): Promise<string> {
    const workflow = this.workflows.get(remoteId);
    if (!workflow) {
      throw new Error('Workflow not found');
    }

    const imported: MarketplaceWorkflow = {
      ...workflow,
      id: `imported_${Date.now()}_${remoteId}`,
      authorId: targetUserId,
      authorName: 'Imported',
      remoteId: `imported_${Date.now()}`,
      downloadCount: 0,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };

    this.workflows.set(imported.remoteId, imported);
    await this.saveToRedis(imported);

    const original = this.workflows.get(remoteId);
    if (original) {
      original.downloadCount += 1;
      await this.saveToRedis(original);
    }

    return imported.workflowJson;
  }

  async rateWorkflow(remoteId: string, userId: string, rating: number, comment?: string): Promise<WorkflowRating> {
    if (rating < 1 || rating > 5) {
      throw new Error('Rating must be between 1 and 5');
    }

    const workflow = this.workflows.get(remoteId);
    if (!workflow) {
      throw new Error('Workflow not found');
    }

    const existingRatings = this.ratings.get(remoteId) || [];
    const existingIndex = existingRatings.findIndex(r => r.userId === userId);

    const newRating: WorkflowRating = {
      id: `rating_${Date.now()}`,
      workflowId: remoteId,
      userId,
      rating,
      comment,
      createdAt: Date.now(),
      moderated: false,
    };

    if (existingIndex >= 0) {
      existingRatings[existingIndex] = newRating;
    } else {
      existingRatings.push(newRating);
    }

    this.ratings.set(remoteId, existingRatings);

    const avgRating = existingRatings.reduce((sum, r) => sum + r.rating, 0) / existingRatings.length;
    workflow.rating = Math.round(avgRating * 10) / 10;
    workflow.ratingCount = existingRatings.length;
    await this.saveToRedis(workflow);

    return newRating;
  }

  async getRatingDistribution(remoteId: string): Promise<Record<number, number>> {
    const ratings = this.ratings.get(remoteId) || [];
    const distribution = { 1: 0, 2: 0, 3: 0, 4: 0, 5: 0 };
    
    for (const r of ratings) {
      if (distribution[r.rating as 1|2|3|4|5] !== undefined) {
        distribution[r.rating as 1|2|3|4|5]++;
      }
    }
    
    return distribution;
  }

  async getRatingStats(remoteId: string): Promise<{
    average: number;
    count: number;
    distribution: Record<number, number>;
  }> {
    const ratings = this.ratings.get(remoteId) || [];
    const distribution = await this.getRatingDistribution(remoteId);
    
    if (ratings.length === 0) {
      return { average: 0, count: 0, distribution };
    }
    
    const avg = ratings.reduce((sum, r) => sum + r.rating, 0) / ratings.length;
    return {
      average: Math.round(avg * 10) / 10,
      count: ratings.length,
      distribution,
    };
  }

  // Version history
  private versions: Map<string, WorkflowVersion[]> = new Map();

  async recordVersion(
    remoteId: string,
    version: number,
    changelog: string,
    authorId: string,
    authorName: string,
    workflowJson?: string,
  ): Promise<void> {
    const versions = this.versions.get(remoteId) || [];
    const diff = versions.length > 0 && workflowJson
      ? this.computeDiff(versions[versions.length - 1].version, workflowJson)
      : undefined;
    
    versions.push({
      version,
      changelog,
      createdAt: Date.now(),
      authorId,
      authorName,
      diff,
    });
    
    this.versions.set(remoteId, versions);
    
    if (workflowJson) {
      await this.redis?.hSet(`wf:${remoteId}:versions`, String(version), JSON.stringify({
        version,
        changelog,
        createdAt: Date.now(),
        authorId,
        authorName,
      }));
    }
  }

  async getVersionHistory(remoteId: string): Promise<WorkflowVersion[]> {
    // Check Redis first
    if (this.redis) {
      const stored = await this.redis.hGetAll(`wf:${remoteId}:versions`);
      if (Object.keys(stored).length > 0) {
        return Object.values(stored)
          .map(v => JSON.parse(v))
          .sort((a, b) => b.version - a.version);
      }
    }
    
    return this.versions.get(remoteId) || [];
  }

  async getVersion(remoteId: string, version: number): Promise<WorkflowVersion | null> {
    const history = await this.getVersionHistory(remoteId);
    return history.find(v => v.version === version) || null;
  }

  private computeDiff(oldVersion: number, newJson: string): string {
    const old = this.versions.get(this.workflows.get(remoteId)?.id || '') || [];
    return `Updated from v${oldVersion}`;
  }

    return newRating;
  }

  async getRatings(remoteId: string): Promise<WorkflowRating[]> {
    return this.ratings.get(remoteId) || [];
  }

  async getUserRating(remoteId: string, userId: string): Promise<WorkflowRating | null> {
    const ratings = this.ratings.get(remoteId) || [];
    return ratings.find(r => r.userId === userId) || null;
  }

  async delete(remoteId: string, userId: string): Promise<void> {
    const workflow = this.workflows.get(remoteId);
    if (!workflow) {
      throw new Error('Workflow not found');
    }

    if (workflow.authorId !== userId) {
      throw new Error('Not authorized');
    }

    this.workflows.delete(remoteId);
    this.ratings.delete(remoteId);

    if (this.redis) {
      await this.redis.del(`marketplace:workflow:${remoteId}`);
    }
  }

  async getTrending(limit: number = 10): Promise<MarketplaceWorkflow[]> {
    const all = Array.from(this.workflows.values());
    return all
      .sort((a, b) => {
        const scoreA = a.rating * Math.log(a.downloadCount + 1);
        const scoreB = b.rating * Math.log(b.downloadCount + 1);
        return scoreB - scoreA;
      })
      .slice(0, limit);
  }

  async getTags(): Promise<{ tag: string; count: number }[]> {
    const tagCounts = new Map<string, number>();
    
    for (const wf of this.workflows.values()) {
      for (const tag of wf.tags) {
        tagCounts.set(tag, (tagCounts.get(tag) || 0) + 1);
      }
    }

    return Array.from(tagCounts.entries())
      .map(([tag, count]) => ({ tag, count }))
      .sort((a, b) => b.count - a.count);
  }

  async getStats(): Promise<{ totalWorkflows: number; totalDownloads: number; avgRating: number }> {
    const all = Array.from(this.workflows.values());
    const totalWorkflows = all.length;
    const totalDownloads = all.reduce((sum, wf) => sum + wf.downloadCount, 0);
    const avgRating = all.length > 0
      ? all.reduce((sum, wf) => sum + wf.rating, 0) / all.length
      : 0;

    return { totalWorkflows, totalDownloads, avgRating: Math.round(avgRating * 10) / 10 };
  }

  async close(): Promise<void> {
    if (this.redis) {
      await this.redis.quit();
    }
  }
}

export function createMarketplace(redisUrl?: string): WorkflowMarketplace {
  return new WorkflowMarketplace(redisUrl);
}