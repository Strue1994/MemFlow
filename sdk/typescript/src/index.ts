import axios from 'axios';

const EXECUTOR_URL = process.env.EXECUTOR_URL || 'http://localhost:8080';

export interface WorkflowStep {
  id?: string;
  type: 'http' | 'code' | 'set' | 'if' | 'for';
  parameters: Record<string, any>;
  name?: string;
}

export interface WorkflowBuilder {
  steps: WorkflowStep[];
  name: string;
  description?: string;
}

export function workflow(name: string): WorkflowBuilder {
  return {
    name,
    steps: [],
  };
}

export function http(config: {
  method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
  url: string;
  headers?: Record<string, string>;
  body?: any;
}): WorkflowStep {
  return {
    type: 'http',
    parameters: {
      method: config.method,
      url: config.url,
      headers: config.headers || {},
      body: config.body,
    },
    name: `${config.method} ${config.url}`,
  };
}

export function set(key: string, value: any): WorkflowStep {
  return {
    type: 'set',
    parameters: { key, value: typeof value === 'function' ? value.toString() : value },
    name: `Set ${key}`,
  };
}

export function code(script: string): WorkflowStep {
  return {
    type: 'code',
    parameters: { code: script },
    name: 'Code',
  };
}

export function ifCondition(condition: string, trueBranch: WorkflowBuilder, falseBranch?: WorkflowBuilder): WorkflowStep {
  return {
    type: 'if',
    parameters: {
      condition,
      trueBranch: trueBranch.steps,
      falseBranch: falseBranch?.steps || [],
    },
    name: 'If',
  };
}

export function forLoop(variable: string, over: any[], body: WorkflowBuilder): WorkflowStep {
  return {
    type: 'for',
    parameters: {
      variable,
      over,
      body: body.steps,
    },
    name: `For ${variable}`,
  };
}

function compileToN8nJson(builder: WorkflowBuilder): any {
  const nodes = builder.steps.map((step, index) => ({
    id: step.id || `node_${index + 1}`,
    name: step.name || `Step ${index + 1}`,
    type: mapStepTypeToN8n(step.type),
    parameters: step.parameters,
    position: [250, index * 120],
  }));

  const connections: Record<string, any> = {};
  for (let i = 0; i < nodes.length - 1; i++) {
    connections[nodes[i].id] = {
      main: [{ node: nodes[i + 1].id, type: 'main', index: 0 }],
    };
  }

  return {
    name: builder.name,
    nodes,
    connections,
  };
}

function mapStepTypeToN8n(type: string): string {
  const map: Record<string, string> = {
    http: 'n8n-nodes-base.httpRequest',
    code: 'n8n-nodes-base.code',
    set: 'n8n-nodes-base.set',
    if: 'n8n-nodes-base.if',
    for: 'n8n-nodes-base.splitInBatches',
  };
  return map[type] || 'n8n-nodes-base.noOp';
}

export async function compileAndUpload(builder: WorkflowBuilder): Promise<{ success: boolean; workflowId?: string; error?: string }> {
  const n8nJson = compileToN8nJson(builder);

  try {
    const response = await axios.post(
      `${EXECUTOR_URL}/compile`,
      {
        name: builder.name,
        description: builder.description,
        n8n_json: n8nJson,
      },
      {
        headers: { 'Content-Type': 'application/json' },
      }
    );

    return {
      success: true,
      workflowId: response.data.workflow_id || response.data.id,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.response?.data?.error || error.message,
    };
  }
}

export async function exportToTypeScript(workflowId: string): Promise<string> {
  const response = await axios.get(`${EXECUTOR_URL}/workflow/${workflowId}`);
  const workflow = response.data;

  let code = `import { workflow, http, set, code } from '@memflow/sdk';\n\n`;
  code += `export default workflow('${workflow.name || workflowId}')\n`;

  for (const node of workflow.nodes || []) {
    if (node.type === 'n8n-nodes-base.httpRequest') {
      code += `  .step(http({ method: 'POST', url: '${node.parameters?.url}' }))\n`;
    } else if (node.type === 'n8n-nodes-base.code') {
      code += `  .step(code('${node.parameters?.code || ''}'))\n`;
    } else if (node.type === 'n8n-nodes-base.set') {
      code += `  .step(set('${node.parameters?.key}', ${JSON.stringify(node.parameters?.value)}))\n`;
    }
  }

  code += `  .export();\n`;
  return code;
}