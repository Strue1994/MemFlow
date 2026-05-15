/**
 * Tool definitions for the AgentLoop.
 * Connects the LLM agent to MemFlow executor, memory, and skills.
 */

import type { Tool } from './agent-loop';
import { globalMCPManager, type MCPTool as MCPToolType } from './mcp-client';

export function createExecutorTools(executorUrl: string, apiKey: string): Tool[] {
  const retryFetch = async (url: string, init: RequestInit, retries = 3): Promise<Response> => {
    const backoff = [100, 500, 2000];
    let lastErr: any;
    for (let i = 0; i < retries; i++) {
      try {
        const resp = await fetch(url, init);
        if (resp.status >= 500 && i < retries - 1) {
          await new Promise((r) => setTimeout(r, backoff[i] || 2000));
          continue;
        }
        return resp;
      } catch (e) {
        lastErr = e;
        if (i < retries - 1) await new Promise((r) => setTimeout(r, backoff[i] || 2000));
      }
    }
    throw lastErr || new Error("fetch failed");
  };

  const exec = async (path: string, body: any): Promise<string> => {
    try {
      const resp = await retryFetch(`${executorUrl}${path}`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-API-Key': apiKey,
        },
        body: JSON.stringify(body),
      });
      if (!resp.ok) return `HTTP ${resp.status}: ${await resp.text()}`;
      return JSON.stringify(await resp.json(), null, 2);
    } catch (err: any) {
      return `Error: ${err.message}`;
    }
  };

  return [
    {
      name: 'execute_workflow',
      description: 'Execute a workflow by ID with optional parameters',
      parameters: {
        type: 'object',
        properties: {
          workflow_id: { type: 'string', description: 'The workflow ID to execute' },
          params: { type: 'object', description: 'Optional parameters for the workflow' },
        },
        required: ['workflow_id'],
      },
      execute: async (args) => exec('/execute', args),
    },
    {
      name: 'list_workflows',
      description: 'List all available workflows',
      parameters: { type: 'object', properties: {} },
      execute: async () => exec('/workflows', {}),
    },
    {
      name: 'create_workflow',
      description: 'Create a new workflow from n8n JSON definition',
      parameters: {
        type: 'object',
        properties: {
          n8n_json: { type: 'object', description: 'The n8n workflow JSON' },
          name: { type: 'string', description: 'Optional workflow name' },
        },
        required: ['n8n_json'],
      },
      execute: async (args) => exec('/compile', args),
    },
  ];
}

export function createMemoryTools(memoryHubUrl: string): Tool[] {
  const retryFetch = async (url: string, init: RequestInit, retries = 3): Promise<Response> => {
    const backoff = [100, 500, 2000];
    let lastErr: any;
    for (let i = 0; i < retries; i++) {
      try {
        const resp = await fetch(url, init);
        if (resp.status >= 500 && i < retries - 1) {
          await new Promise((r) => setTimeout(r, backoff[i] || 2000));
          continue;
        }
        return resp;
      } catch (e) {
        lastErr = e;
        if (i < retries - 1) await new Promise((r) => setTimeout(r, backoff[i] || 2000));
      }
    }
    throw lastErr || new Error("fetch failed");
  };

  const mem = async (path: string, body?: any): Promise<string> => {
    try {
      const opts: RequestInit = {
        method: body ? 'POST' : 'GET',
        headers: { 'Content-Type': 'application/json' },
      };
      if (body) opts.body = JSON.stringify(body);
      const resp = await retryFetch(`${memoryHubUrl}${path}`, opts);
      if (!resp.ok) return `HTTP ${resp.status}`;
      return JSON.stringify(await resp.json(), null, 2);
    } catch (err: any) {
      return `Error: ${err.message}`;
    }
  };

  return [
    {
      name: 'search_memory',
      description: 'Search stored memories by semantic query',
      parameters: {
        type: 'object',
        properties: {
          query: { type: 'string', description: 'Search query' },
          k: { type: 'number', description: 'Number of results (default 5)' },
        },
        required: ['query'],
      },
      execute: async (args) => mem(`/memories/search?q=${encodeURIComponent(args.query as string)}&k=${args.k || 5}`),
    },
    {
      name: 'store_memory',
      description: 'Store a new memory entry',
      parameters: {
        type: 'object',
        properties: {
          content: { type: 'string', description: 'Memory content' },
          type: { type: 'string', description: 'Memory type (UserPreference|WorkflowPattern|ErrorRecovery|Conversation)' },
          importance: { type: 'number', description: 'Importance 0.0-1.0' },
        },
        required: ['content'],
      },
      execute: async (args) => mem('/memories', args),
    },
  ];
}

export function createSkillTools(): Tool[] {
  return [
    {
      name: 'create_skill',
      description: 'Create a reusable skill from task execution',
      parameters: {
        type: 'object',
        properties: {
          name: { type: 'string', description: 'Skill name' },
          description: { type: 'string', description: 'What the skill does' },
          pattern: { type: 'string', description: 'Execution pattern steps' },
        },
        required: ['name', 'description', 'pattern'],
      },
      execute: async (args) => {
        return JSON.stringify({ status: 'skill_created', name: args.name });
      },
    },
    {
      name: 'list_skills',
      description: 'List all available skills in the skill manager',
      parameters: { type: 'object', properties: {} },
      execute: async () => {
        const { SkillManager } = require('./skill-system');
        const sm = new SkillManager();
        return JSON.stringify(sm.listSkills().map((s: any) => ({ name: s.name, description: s.description })));
      },
    },
  ];
}

/** Build Tool[] from connected MCP servers */
export function createMCPTools(): Tool[] {
  const mcpTools: MCPToolType[] = globalMCPManager.getAllTools();
  return mcpTools.map((mt) => ({
    name: `mcp_${mt.serverName}_${mt.name}`,
    description: `[MCP/${mt.serverName}] ${mt.description}`,
    parameters: (mt.inputSchema as any) || { type: 'object', properties: {} },
    execute: async (args) => {
      try {
        const conn = globalMCPManager.getConnections().find((c) => c.config.name === mt.serverName);
        if (!conn) return `MCP server "${mt.serverName}" not connected`;
        return await conn.callTool(mt.name, args);
      } catch (err: any) {
        return `MCP tool error: ${err.message}`;
      }
    },
  }));
}
