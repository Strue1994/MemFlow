/**
 * T1.3: MCP Server — Exposes MemFlow as an MCP-compatible server.
 * 
 * Protocol: https://modelcontextprotocol.io
 * Transports: stdio (for IDE integration) and SSE (for remote access)
 * 
 * Exposed Tools:
 * - execute_workflow: Run a workflow by ID
 * - search_memory: Semantic memory search
 * - list_skills: List available skills
 * - create_skill: Create a new skill
 */

import * as http from 'node:http';

export interface MCPServerConfig {
  transport: 'stdio' | 'sse';
  port?: number;
  allowedTools?: string[];
  executorUrl?: string;
  memoryHubUrl?: string;
}

interface MCPRequest {
  jsonrpc: '2.0';
  id: number;
  method: string;
  params?: any;
}

interface MCPResponse {
  jsonrpc: '2.0';
  id: number;
  result?: any;
  error?: { code: number; message: string };
}

// ---- Tool Definitions ----

interface MCPTool {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
  handler: (params: any) => Promise<any>;
}

function createTools(config: MCPServerConfig): MCPTool[] {
  const tools: MCPTool[] = [
    {
      name: 'execute_workflow',
      description: 'Execute a workflow by ID with optional parameters',
      inputSchema: {
        type: 'object',
        properties: {
          workflow_id: { type: 'string' },
          params: { type: 'object' },
        },
        required: ['workflow_id'],
      },
      handler: async (params) => {
        const url = config.executorUrl || 'http://localhost:8082';
        const resp = await fetch(`${url}/execute`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(params),
        });
        return resp.json();
      },
    },
    {
      name: 'search_memory',
      description: 'Search stored memories by semantic query',
      inputSchema: {
        type: 'object',
        properties: {
          query: { type: 'string' },
          k: { type: 'number' },
        },
        required: ['query'],
      },
      handler: async (params) => {
        const url = config.memoryHubUrl || 'http://localhost:8081';
        const resp = await fetch(`${url}/memories/search?q=${encodeURIComponent(params.query)}&k=${params.k || 5}`);
        return resp.json();
      },
    },
    {
      name: 'list_skills',
      description: 'List all available skills from the skills directory',
      inputSchema: { type: 'object', properties: {} },
      handler: async () => {
        const fs = await import('node:fs');
        const path = await import('node:path');
        const dir = path.resolve(process.cwd(), '..', 'skills');
        if (!fs.existsSync(dir)) return { skills: [] };
        const files = fs.readdirSync(dir).filter((f) => f.endsWith('.json') || f.endsWith('.md'));
        return { skills: files };
      },
    },
  ];

  if (config.allowedTools) {
    return tools.filter((t) => config.allowedTools!.includes(t.name));
  }
  return tools;
}

// ---- MCP Server ----

export class MCPServer {
  private config: MCPServerConfig;
  private tools: MCPTool[];
  private server: http.Server | null = null;

  constructor(config: MCPServerConfig) {
    this.config = config;
    this.tools = createTools(config);
  }

  async start(): Promise<boolean> {
    if (this.config.transport === 'stdio') {
      return this.startStdio();
    }
    return this.startSSE();
  }

  private async startStdio(): Promise<boolean> {
    process.stdin.setEncoding('utf-8');
    let buffer = '';

    process.stdin.on('data', (chunk: string) => {
      buffer += chunk;
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        try {
          const req: MCPRequest = JSON.parse(trimmed);
          this.handleRequest(req).then((resp) => {
            process.stdout.write(JSON.stringify(resp) + '\n');
          });
        } catch {
          // Ignore malformed JSON
        }
      }
    });

    // Send initialize response
    const initResp: MCPResponse = {
      jsonrpc: '2.0',
      id: 0,
      result: {
        protocolVersion: '2025-03-26',
        capabilities: {
          tools: {},
          resources: {},
        },
        serverInfo: {
          name: 'memflow-mcp',
          version: '0.1.0',
        },
      },
    };
    process.stdout.write(JSON.stringify(initResp) + '\n');

    return true;
  }

  private async startSSE(): Promise<boolean> {
    const port = this.config.port || 3301;

    this.server = http.createServer(async (req, res) => {
      if (req.method === 'POST') {
        let body = '';
        req.on('data', (chunk) => { body += chunk; });
        req.on('end', async () => {
          try {
            const mcpReq: MCPRequest = JSON.parse(body);
            const resp = await this.handleRequest(mcpReq);
            res.writeHead(200, { 'Content-Type': 'application/json' });
            res.end(JSON.stringify(resp));
          } catch {
            res.writeHead(400);
            res.end(JSON.stringify({ error: 'Invalid request' }));
          }
        });
      } else {
        res.writeHead(200, { 'Content-Type': 'text/plain' });
        res.end('MemFlow MCP Server');
      }
    });

    return new Promise((resolve) => {
      this.server!.listen(port, () => {
        console.log(`MCP Server (SSE) listening on port ${port}`);
        resolve(true);
      });
    });
  }

  private async handleRequest(req: MCPRequest): Promise<MCPResponse> {
    switch (req.method) {
      case 'tools/list': {
        return {
          jsonrpc: '2.0',
          id: req.id,
          result: {
            tools: this.tools.map((t) => ({
              name: t.name,
              description: t.description,
              inputSchema: t.inputSchema,
            })),
          },
        };
      }

      case 'tools/call': {
        const tool = this.tools.find((t) => t.name === req.params?.name);
        if (!tool) {
          return {
            jsonrpc: '2.0',
            id: req.id,
            error: { code: -32601, message: `Tool not found: ${req.params?.name}` },
          };
        }
        try {
          const result = await tool.handler(req.params?.arguments);
          return {
            jsonrpc: '2.0',
            id: req.id,
            result: { content: [{ type: 'text', text: JSON.stringify(result) }] },
          };
        } catch (err: any) {
          return {
            jsonrpc: '2.0',
            id: req.id,
            error: { code: -32000, message: err.message },
          };
        }
      }

      case 'initialize':
        return {
          jsonrpc: '2.0',
          id: req.id,
          result: {
            protocolVersion: '2025-03-26',
            capabilities: { tools: {}, resources: {} },
            serverInfo: { name: 'memflow-mcp', version: '0.1.0' },
          },
        };

      default:
        return {
          jsonrpc: '2.0',
          id: req.id,
          error: { code: -32601, message: `Method not found: ${req.method}` },
        };
    }
  }

  stop(): void {
    if (this.server) {
      this.server.close();
    }
  }
}
