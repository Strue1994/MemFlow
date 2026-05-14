export interface WorkflowNode {
  id: string;
  type: string;
  parameters: Record<string, unknown>;
}

export interface Workflow {
  id: string;
  nodes: WorkflowNode[];
}

export const workflowRegistry: Map<string, Workflow> = new Map([
  [
    "wf_zen",
    {
      id: "wf_zen",
      nodes: [
        {
          id: "1",
          type: "n8n-nodes-base.httpRequest",
          parameters: {
            url: "https://api.github.com/zen",
            method: "GET",
          },
        },
      ],
    },
  ],
  [
    "wf_add",
    {
      id: "wf_add",
      nodes: [
        {
          id: "1",
          type: "n8n-nodes-base.set",
          parameters: { values: { a: 2 } },
        },
        {
          id: "2",
          type: "n8n-nodes-base.set",
          parameters: { values: { b: 3 } },
        },
      ],
    },
  ],
]);

export function getWorkflow(id: string): Workflow | undefined {
  return workflowRegistry.get(id);
}

export function listWorkflows(): string[] {
  return Array.from(workflowRegistry.keys());
}

export function registerWorkflow(id: string, workflow: Workflow): void {
  workflowRegistry.set(id, workflow);
}