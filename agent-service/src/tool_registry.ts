import { getWorkflow, listWorkflows } from './workflowRegistry';

export interface HermesTool {
  name: string;
  description: string;
  parameters?: {
    type: 'object';
    properties: Record<string, any>;
    required?: string[];
  };
  workflowId?: string;
}

export interface HermesToolRegistry {
  tools: Map<string, HermesTool>;
}

export class ToolRegistry {
  private tools: Map<string, HermesTool> = new Map();

  constructor() {
    this.loadFromWorkflows();
  }

  private loadFromWorkflows(): void {
    const workflows = listWorkflows();
    
    for (const wf of workflows) {
      const workflow = getWorkflow(wf);
      if (!workflow) continue;

      const params = this.extractParameters(workflow);
      
      this.tools.set(`memflow_workflow_${wf}`, {
        name: `memflow_workflow_${wf}`,
        description: this.extractDescription(workflow),
        parameters: params,
        workflowId: wf,
      });
    }
  }

  private extractParameters(workflow: any): any {
    const inputSchema = workflow?.metadata?.input_schema;
    if (inputSchema) {
      return inputSchema;
    }

    const params: Record<string, any> = {};
    if (workflow?.nodes) {
      for (const node of workflow.nodes) {
        if (node.parameters) {
          for (const [key, value] of Object.entries(node.parameters)) {
            if (typeof value === 'string' && value.startsWith('{{')) {
              params[key] = { type: 'string', description: `Parameter: ${key}` };
            }
          }
        }
      }
    }

    return {
      type: 'object',
      properties: params,
      required: [],
    };
  }

  private extractDescription(workflow: any): string {
    return workflow?.name || workflow?.id || 'MemFlow workflow';
  }

  registerTool(tool: HermesTool): void {
    this.tools.set(tool.name, tool);
  }

  getTool(name: string): HermesTool | undefined {
    return this.tools.get(name);
  }

  getAllTools(): HermesTool[] {
    return Array.from(this.tools.values());
  }

  refresh(): void {
    this.loadFromWorkflows();
  }
}

export const toolRegistry = new ToolRegistry();
