import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import axios from "axios";

const MEMFLOW_API_URL = process.env.MEMFLOW_API_URL || "http://localhost:3000";
const MEMFLOW_API_KEY = process.env.MEMFLOW_API_KEY || "";

interface ToolArgument {
  [key: string]: any;
}

class MemFlowServer {
  private server: Server;
  private apiClient: ReturnType<typeof axios.create>;

  constructor() {
    this.server = new Server(
      {
        name: "memflow-mcp",
        version: "1.0.0",
      },
      {
        capabilities: {
          tools: {},
        },
      }
    );

    this.apiClient = axios.create({
      baseURL: MEMFLOW_API_URL,
      headers: MEMFLOW_API_KEY
        ? { Authorization: `Bearer ${MEMFLOW_API_KEY}`, "Content-Type": "application/json" }
        : { "Content-Type": "application/json" },
      timeout: 60000,
    });

    this.setupHandlers();
  }

  private setupHandlers(): void {
    this.server.setRequestHandler(ListToolsRequestSchema, async () => {
      return {
        tools: [
          {
            name: "memflow_create_workflow",
            description: "根据自然语言描述创建一个新的 n8n 工作流",
            inputSchema: {
              type: "object",
              properties: {
                description: {
                  type: "string",
                  description: "工作流的自然语言描述，例如：'创建一个每天早上9点抓取RSS并推送飞书的工作流'",
                },
              },
              required: ["description"],
            },
          },
          {
            name: "memflow_execute_workflow",
            description: "执行一个已存在的工作流",
            inputSchema: {
              type: "object",
              properties: {
                workflow_id: {
                  type: "string",
                  description: "工作流 ID",
                },
                params: {
                  type: "object",
                  description: "执行参数（可选）",
                },
              },
              required: ["workflow_id"],
            },
          },
          {
            name: "memflow_list_workflows",
            description: "列出所有工作流",
            inputSchema: {
              type: "object",
              properties: {},
            },
          },
          {
            name: "memflow_get_workflow",
            description: "获取工作流详情",
            inputSchema: {
              type: "object",
              properties: {
                workflow_id: {
                  type: "string",
                  description: "工作流 ID",
                },
              },
              required: ["workflow_id"],
            },
          },
          {
            name: "memflow_validate_workflow",
            description: "验证工作流 JSON 配置",
            inputSchema: {
              type: "object",
              properties: {
                workflow_json: {
                  type: "object",
                  description: "n8n 工作流 JSON",
                },
              },
              required: ["workflow_json"],
            },
          },
          {
            name: "memflow_submit_feedback",
            description: "提交用户反馈以改进模式匹配",
            inputSchema: {
              type: "object",
              properties: {
                pattern_id: {
                  type: "string",
                  description: "模式 ID",
                },
                user_request: {
                  type: "string",
                  description: "用户请求",
                },
                accepted: {
                  type: "boolean",
                  description: "是否接受推荐",
                },
                modifications: {
                  type: "object",
                  description: "用户修改内容（如果有）",
                },
              },
              required: ["pattern_id", "user_request", "accepted"],
            },
          },
        ],
      };
    });

    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params as {
        name: string;
        arguments: ToolArgument;
      };

      try {
        switch (name) {
          case "memflow_create_workflow":
            return await this.createWorkflow(args.description as string);
          case "memflow_execute_workflow":
            return await this.executeWorkflow(
              args.workflow_id as string,
              args.params as Record<string, any>
            );
          case "memflow_list_workflows":
            return await this.listWorkflows();
          case "memflow_get_workflow":
            return await this.getWorkflow(args.workflow_id as string);
          case "memflow_validate_workflow":
            return await this.validateWorkflow(args.workflow_json as object);
          case "memflow_submit_feedback":
            return await this.submitFeedback(
              args.pattern_id as string,
              args.user_request as string,
              args.accepted as boolean,
              args.modifications as object | undefined
            );
          default:
            throw new Error(`Unknown tool: ${name}`);
        }
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        console.error(`[MemFlow MCP] Error: ${message}`);
        return {
          content: [{ type: "text", text: `Error: ${message}` }],
          isError: true,
        };
      }
    });
  }

  private async createWorkflow(description: string): Promise<any> {
    console.log(`[MemFlow MCP] Creating workflow: ${description}`);

    try {
      const response = await this.apiClient.post("/create_workflow_v2", {
        user_request: description,
        step: 1,
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(response.data, null, 2),
          },
        ],
      };
    } catch (error: any) {
      const message = error.response?.data?.error || error.message;
      throw new Error(`Failed to create workflow: ${message}`);
    }
  }

  private async executeWorkflow(
    workflowId: string,
    params?: Record<string, any>
  ): Promise<any> {
    console.log(`[MemFlow MCP] Executing workflow: ${workflowId}`);

    try {
      const response = await this.apiClient.post("/execute", {
        workflowId,
        params: params || {},
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(response.data, null, 2),
          },
        ],
      };
    } catch (error: any) {
      const message = error.response?.data?.error || error.message;
      throw new Error(`Failed to execute workflow: ${message}`);
    }
  }

  private async listWorkflows(): Promise<any> {
    console.log(`[MemFlow MCP] Listing workflows`);

    try {
      const response = await this.apiClient.get("/workflows");

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(response.data, null, 2),
          },
        ],
      };
    } catch (error: any) {
      const message = error.response?.data?.error || error.message;
      throw new Error(`Failed to list workflows: ${message}`);
    }
  }

  private async getWorkflow(workflowId: string): Promise<any> {
    console.log(`[MemFlow MCP] Getting workflow: ${workflowId}`);

    try {
      const response = await this.apiClient.get(`/workflows/${workflowId}`);

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(response.data, null, 2),
          },
        ],
      };
    } catch (error: any) {
      const message = error.response?.data?.error || error.message;
      throw new Error(`Failed to get workflow: ${message}`);
    }
  }

  private async validateWorkflow(workflowJson: object): Promise<any> {
    console.log(`[MemFlow MCP] Validating workflow`);

    try {
      const response = await this.apiClient.post("/validate", {
        n8n_json: workflowJson,
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(response.data, null, 2),
          },
        ],
      };
    } catch (error: any) {
      const message = error.response?.data?.error || error.message;
      throw new Error(`Failed to validate workflow: ${message}`);
    }
  }

  private async submitFeedback(
    patternId: string,
    userRequest: string,
    accepted: boolean,
    modifications?: object
  ): Promise<any> {
    console.log(`[MemFlow MCP] Submitting feedback: ${patternId}, accepted: ${accepted}`);

    try {
      const response = await this.apiClient.post("/feedback", {
        pattern_id: patternId,
        user_request: userRequest,
        accepted,
        modifications,
      });

      return {
        content: [
          {
            type: "text",
            text: JSON.stringify(response.data, null, 2),
          },
        ],
      };
    } catch (error: any) {
      const message = error.response?.data?.error || error.message;
      throw new Error(`Failed to submit feedback: ${message}`);
    }
  }

  async start(): Promise<void> {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.log("[MemFlow MCP] Server started");
  }
}

const server = new MemFlowServer();
server.start().catch(console.error);