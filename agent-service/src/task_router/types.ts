export type TaskRoute = "workflow" | "generated_workflow" | "agent" | "clarification";

export interface WorkflowAssetMetadata {
  workflowId: string;
  description: string;
  inputHints: string[];
  outputType: string;
  successRate: number;
  reusable: boolean;
  examplePrompts: string[];
  failureCategories: string[];
  updatedAt: string;
}

export interface HistoricalTaskRecord {
  taskText: string;
  route: TaskRoute;
  workflowId?: string;
  success: boolean;
  parameterKeys: string[];
  outputType: string;
  createdAt: string;
}

export interface RoutingDecision {
  route: TaskRoute;
  repeatable: boolean;
  confidence: "high" | "medium" | "low";
  reason: string;
  workflowId?: string;
  clarificationQuestion?: string;
}
