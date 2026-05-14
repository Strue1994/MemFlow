# MemFlow Workflow-First Agent Platform Design

Date: 2026-04-30
Status: Draft for review

## 1. Goal

MemFlow first-stage product goal is:

Build a local-first general agent automation platform where natural-language tasks enter through one unified console, the system decides whether the task is repeatable or one-off, then routes execution to workflow, agent, or agent-generated workflow, and always returns an explainable result.

This is not a pure chat agent product and not a pure workflow editor. It is a workflow-first automation platform with agent-driven routing, generation, and explanation.

## 2. Product Positioning

### Core product statement

MemFlow should be positioned as:

- A local runnable automation work platform
- Workflow-centered for repeatable work
- Agent-assisted for understanding, routing, and one-off tasks
- Single-user, single-machine first stage, with structure reserved for later expansion

### Primary success path

First-stage primary path:

`Natural language request -> determine repeatable vs one-off -> route to workflow or agent -> execute -> explain result -> optionally save reusable workflow asset`

### First-stage success standard

Stage 1 must optimize for:

- Stable execution of matched existing workflows
- Clear explanations when execution fails or cannot be routed confidently
- Automatic workflow generation and execution for repeatable tasks without existing workflow coverage
- Direct agent handling for one-off tasks

It does not need to optimize first for:

- Multi-user collaboration
- Open-ended autonomy
- Marketplace growth
- Visible multi-agent orchestration
- Advanced optimization research features

## 3. Task Routing Model

### Core routing rule

The routing decision is:

- Repeatable task -> workflow path
- One-off task -> agent path
- Repeatable task with no matching workflow -> generate workflow, then execute it
- Uncertain task -> ask the user a targeted clarification question before routing

### Definition of repeatable task

Repeatable task should be determined in two stages:

1. Semantic candidate recall
2. Structural and historical confirmation

A task is considered repeatable when the system finds strong evidence that:

- The current request is semantically similar to prior successful tasks or workflow descriptions
- The expected input and output shape is similar
- The task pattern is likely to be reused
- The routing confidence is high enough to avoid unnecessary agent-only handling

### Routing confidence levels

- High confidence repeatable: route directly to workflow
- Medium confidence repeatable: ask one short user clarification if needed
- Low confidence or clearly one-off: route to agent
- High confidence repeatable but no matching workflow asset: auto-generate workflow and execute it

### Routing explanation requirement

Every task execution must produce a routing explanation such as:

- Matched an existing workflow and executed it
- Did not match an existing workflow, generated a new workflow, and executed it
- Detected a one-off task and routed it to agent execution
- Could not determine route with enough confidence and requested clarification

## 4. First-Stage User Flow

### Main flow

1. User enters a natural-language task.
2. System analyzes whether the task is repeatable.
3. If repeatable, system recalls workflow candidates, historical executions, and successful patterns.
4. System checks structural fit:
   - Required inputs
   - Expected outputs
   - Prior success rate
   - Known failure patterns
5. System routes the task:
   - Existing workflow
   - Newly generated workflow
   - Direct agent handling
   - Clarification question
6. System executes and returns:
   - Route used
   - Reason for route
   - What was executed
   - Success or failure
   - Recovery guidance
7. If the result is reusable, system offers to save or promote it as workflow asset.

### Clarification flow

When route confidence is not high enough, the system asks a single focused question that fills a missing execution-critical gap. The question must explain:

- What is missing
- Why it is required
- What route will be taken after the answer

## 5. System Architecture

### Target responsibilities

- `executor`: workflow execution kernel
- `agent-service`: natural-language entry, routing, generation, explanation
- `web-ui`: task control console, result visibility, workflow asset management
- `learning-engine` and `memory-hub`: evidence and pattern support for routing and asset improvement
- Python runtime: experimental path only, not product-critical path

### Component boundaries

#### 5.1 Executor

`executor` remains the single execution kernel for workflow runs. It should own:

- Workflow execution
- Runtime state
- Persistence
- Logs
- Failure recording
- Retry and recovery data

It should not own:

- Natural-language routing
- Repeatability detection
- Multi-agent reasoning
- User clarification policy

#### 5.2 Agent Service

`agent-service` becomes the real product entrypoint. It should own:

- Natural-language intake
- Repeatable vs one-off classification
- Workflow candidate recall
- Agent vs workflow route choice
- New workflow generation when needed
- Result explanation
- Clarification questions
- Post-run workflow asset decisions

#### 5.3 Web UI

`web-ui` becomes the task-driven control console. It should own:

- Main task entry
- Routing explanation display
- Execution progress display
- Result and recovery display
- Workflow asset management
- Settings and execution policy controls

#### 5.4 Python runtime

`main.py` and `core/agent_loop.py` should be explicitly repositioned as:

- Experimental runtime
- Prototype area
- Future alternative agent research lane

They should not remain on the first-stage user-facing product path.

## 6. Feature Priorities

### Must build in Stage 1

#### 6.1 Task router

A dedicated task-routing module must be introduced or formalized. It should handle:

- Repeatability assessment
- Workflow candidate retrieval
- Route decision
- Route explanation
- Clarification fallback

#### 6.2 Unified execution result model

All execution paths must return one common structure containing at least:

- Route selected
- Route reason
- Workflow id or generated workflow summary when applicable
- Success or failure
- Failure category
- Missing parameter hints
- Suggested next action

#### 6.3 Workflow asset metadata

Each workflow asset should carry:

- Human-readable task description
- Input schema
- Output type
- Last successful runs
- Recent failure patterns
- Reusability status
- Matching hints

#### 6.4 Historical task evidence layer

The system must store and reuse:

- Semantically similar historical tasks
- Successful route choices
- Frequent missing parameters
- Agent-only one-off patterns
- Workflow generation outcomes

#### 6.5 Failure classification

Failures should be grouped into at least:

- Missing parameters
- Wrong workflow match
- Workflow execution failure
- Environment unavailable
- External dependency failure
- Requires user confirmation

### Deprioritize in Stage 1 but preserve for later

- Autonomy as a default experience
- Marketplace-led growth
- User-visible multi-agent interface
- Timeline-first learning UI
- RL, federated learning, or hyperparameter optimization as product priorities

These remain part of the longer-term roadmap but must not dominate the first-stage critical path.

## 7. Enhancement Strategy for Stage 2+

### Learning

Learning should first improve:

- Repeatability detection
- Clarification quality
- Workflow ranking
- Workflow generation quality
- Workflow promotion decisions

### Multi-agent

Multi-agent should be used only for:

- Complex one-off tasks that need coordinated investigation and verification
- Workflow generation and validation pipelines for repeatable tasks without existing assets

It should remain an internal capability before it becomes a user-facing product concept.

### Self-evolution

Self-evolution should mean workflow asset evolution, not unconstrained system mutation. It should focus on:

- Promoting successful patterns into workflow assets
- Lowering ranking of unreliable workflows
- Suggesting repairs for repeatedly failing workflows
- Improving question templates for missing inputs

## 8. Frontend Product Design

### Information hierarchy

The frontend should follow this order:

1. Task
2. System judgment
3. Execution process
4. Workflow asset
5. Advanced features

### Page structure

#### 8.1 Main page: Task Console

The homepage should become a single task console with:

- Task input
- Routing explanation card
- Execution timeline
- Result and recovery panel

#### 8.2 Supporting page: Workflow Assets

This page should manage reusable workflows and generated workflows:

- Asset list
- Suitability and metadata
- Success and failure signals
- Manual correction and saving

#### 8.3 Supporting page: Execution History

This page should show:

- Original task
- Route taken
- What executed
- Outcome
- Failure category
- Whether it was promoted to workflow asset

#### 8.4 Supporting page: Settings

This page should control:

- Model configuration
- Execution policy
- Risk tolerance
- Local capability toggles
- Learning and autonomy switches

#### 8.5 Advanced area

These should be preserved but demoted:

- Timeline
- Learning reports
- Marketplace
- Autonomy
- Optimization tools

### High-value interactions

The UI should add:

- Routing explanation cards
- Single-turn clarification prompts
- Execution preview before side effects
- Structured failure recovery panels
- Workflow save/promotion suggestions
- Quick access to recent successful tasks and similar historical tasks

## 9. Existing Codebase Direction

### Preserve and elevate

- `executor` as execution core
- `agent-service` as real entrypoint
- `WorkflowEditor` as workflow asset editing tool
- `ExecutionLogs` as the base for execution history
- `Settings` as strategy and policy control
- `Layout` as visual shell, with new navigation priorities

### Demote or absorb

- `Dashboard` into the task console overview
- `NLCreator` into the task console input path
- `ComputerAgent` into the agent execution capability surface
- `EvolutionTimeline` and `Marketplace` into the advanced area

### Clean up

- Remove duplicate source tree drift such as `web-ui/src/src`
- Remove overlapping first-stage entry points that duplicate the task console purpose
- Split oversized UI and service files into task, workflow, history, and settings domains

## 10. Error Handling and Safety

### Execution-side effects

The current product direction allows broad direct execution, but the UI and service must still make side effects visible. For workflow or agent execution, the system should show whether the run will:

- Read data only
- Modify local files
- Run shell commands
- Call external services
- Perform destructive actions

### Safety behavior

Stage 1 may execute broadly, but must still retain:

- Visible previews where possible
- Failure explanation
- Clarification when intent is ambiguous
- Recovery options when route choice was wrong

## 11. Testing Strategy

### Required coverage

The design should lead to tests for:

- Repeatable vs one-off route classification
- Clarification fallback logic
- Existing workflow match path
- Generated workflow path
- Direct agent path
- Unified result payload shape
- Failure classification and recovery hints

### Suggested verification layers

- Unit tests for route decision logic
- Integration tests for `agent-service -> executor` route outcomes
- Workflow execution tests in executor
- UI tests for task console, routing explanation, and recovery flows

## 12. Phased Roadmap

### Stage 1

- Establish task console
- Build explicit task router
- Standardize route/result model
- Promote `agent-service` to true entrypoint
- Keep executor as workflow core
- Reposition Python runtime as experimental
- Reduce UI sprawl and clean duplicate source structure

### Stage 2

- Improve routing through learned historical evidence
- Improve workflow asset quality signals
- Add better promotion and repair flows for workflows
- Add controlled multi-agent handling for complex one-off tasks and workflow generation

### Stage 3

- Add learning timeline and deeper review tools
- Add controlled autonomy modes
- Add marketplace and advanced optimization back into the product as secondary enhancements

## 13. Decision Summary

The product should be built around one decisive rule:

`Repeatable tasks go to workflows. One-off tasks go to agents. If a repeatable task has no matching workflow, MemFlow generates one and executes it. If the system is not sure, it asks the user a focused question.`

All architecture, UI, learning, and enhancement work should support that rule.
