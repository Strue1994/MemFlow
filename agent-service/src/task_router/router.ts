import { overlapScore, tokenize } from "./scorer";
import type { HistoricalTaskRecord, RoutingDecision, WorkflowAssetMetadata } from "./types";

const EXPLORATORY_PATTERNS = [
  "analyze",
  "investigate",
  "review",
  "look through",
  "why",
  "unusual",
  "分析",
  "调查",
  "排查",
  "看看",
  "查看",
  "为什么",
  "异常",
  "不对劲",
];

function buildWorkflowCorpus(workflow: WorkflowAssetMetadata): string {
  return [
    workflow.description,
    workflow.outputType,
    ...workflow.inputHints,
    ...workflow.examplePrompts,
    ...workflow.failureCategories,
  ]
    .filter(Boolean)
    .join(" ");
}

function buildMissingInputsQuestion(taskText: string, workflow?: WorkflowAssetMetadata): string {
  if (!workflow) {
    return "What system, files, or date range should be used? I am missing the core routing inputs.";
  }

  const taskTokens = new Set(tokenize(taskText));
  const missingHints = workflow.inputHints.filter((hint) => {
    const hintTokens = tokenize(hint);
    return hintTokens.length > 0 && hintTokens.some((token) => !taskTokens.has(token));
  });

  if (missingHints.length > 0) {
    return `I am missing required inputs for workflow ${workflow.workflowId}: ${missingHints.join(", ")}.`;
  }

  return `I am missing enough detail to run workflow ${workflow.workflowId}. Which system, files, or date range should be used?`;
}

function isExploratoryTask(taskText: string): boolean {
  const lowered = taskText.toLowerCase();
  return EXPLORATORY_PATTERNS.some((pattern) => lowered.includes(pattern));
}

export function decideTaskRoute({
  taskText,
  workflows,
  history,
}: {
  taskText: string;
  workflows: WorkflowAssetMetadata[];
  history: HistoricalTaskRecord[];
}): RoutingDecision {
  let bestWorkflow: WorkflowAssetMetadata | undefined;
  let bestWorkflowScore = 0;

  for (const workflow of workflows) {
    const score = overlapScore(taskText, buildWorkflowCorpus(workflow));
    if (score > bestWorkflowScore) {
      bestWorkflowScore = score;
      bestWorkflow = workflow;
    }
  }

  let bestHistory: HistoricalTaskRecord | undefined;
  let bestHistoryScore = 0;

  for (const entry of history) {
    const score = overlapScore(taskText, entry.taskText);
    if (score > bestHistoryScore) {
      bestHistoryScore = score;
      bestHistory = entry;
    }
  }

  if (bestWorkflow && bestWorkflow.reusable && bestWorkflowScore >= 0.6 && bestWorkflow.successRate >= 0.7) {
    return {
      route: "workflow",
      repeatable: true,
      confidence: "high",
      workflowId: bestWorkflow.workflowId,
      reason: `Matched reusable workflow ${bestWorkflow.workflowId} with deterministic overlap ${bestWorkflowScore.toFixed(2)} and success rate ${bestWorkflow.successRate.toFixed(2)}.`,
    };
  }

  if (
    bestHistory &&
    bestHistoryScore >= 0.6 &&
    bestHistory.success &&
    (!bestWorkflow || bestWorkflowScore < 0.6 || !bestWorkflow.reusable)
  ) {
    return {
      route: "generated_workflow",
      repeatable: true,
      confidence: "high",
      reason: `Matched successful history with overlap ${bestHistoryScore.toFixed(2)} but no reusable workflow asset is available.`,
    };
  }

  if (isExploratoryTask(taskText)) {
    return {
      route: "agent",
      repeatable: false,
      confidence: "high",
      reason: "Exploratory language indicates a one-off investigation better handled by an agent.",
    };
  }

  const bestScore = Math.max(bestWorkflowScore, bestHistoryScore);
  if (bestScore >= 0.35) {
    return {
      route: "clarification",
      repeatable: true,
      confidence: "medium",
      reason: `Partial deterministic match detected with overlap ${bestScore.toFixed(2)}, but required inputs are still missing.`,
      workflowId: bestWorkflow?.workflowId,
      clarificationQuestion: buildMissingInputsQuestion(taskText, bestWorkflow),
    };
  }

  return {
    route: "clarification",
    repeatable: false,
    confidence: "low",
    reason: "No strong reusable pattern was detected from workflow assets or task history.",
    clarificationQuestion: "What system, files, or date range should be used for this task?",
  };
}
