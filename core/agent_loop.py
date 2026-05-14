"""MemFlow Agent Loop - Core decision making logic with checkpoint/retry"""

import asyncio
import copy
from typing import Any, AsyncGenerator, Dict, List, Optional
from dataclasses import dataclass, field
from enum import Enum

from .tool_manager import ToolManager
from .task_planner import TaskPlanner, SubGoal, Plan
from utils.context_manager import ContextManager


class ActionType(Enum):
    """Possible actions the agent can take"""
    RESPOND = "respond"
    CALL_TOOL = "call_tool"
    ASK_CLARIFICATION = "ask_clarification"


@dataclass
class ToolCall:
    """Represents a tool call to be executed"""
    name: str
    arguments: dict = field(default_factory=dict)
    id: Optional[str] = None


@dataclass  
class Thought:
    """Represents the LLM's thought process"""
    action: ActionType
    content: str = ""
    tool_call: Optional[ToolCall] = None


@dataclass
class Checkpoint:
    """Snapshot of agent state for recovery"""
    step_index: int
    messages_snapshot: List[Dict[str, str]]
    intermediate_results: Dict[str, Any] = field(default_factory=dict)
    plan_snapshot: Optional[Plan] = None


class AgentLoop:
    """
    Core agent loop that orchestrates thinking, acting, and observing.
    
    Enhanced with:
    - Checkpoint/retry for fault tolerance
    - Task decomposition via TaskPlanner
    - Dynamic replanning on persistent failures
    
    Attributes:
        tool_manager: Manager for registering and calling tools
        context: Context manager for conversation history
        max_iterations: Maximum number of loops before forcing response
        max_retries_per_step: Max retries before replanning
    """

    def __init__(
        self,
        tool_manager: ToolManager,
        context: Optional[ContextManager] = None,
        max_iterations: int = 10,
        max_retries_per_step: int = 2,
        planner: Optional[TaskPlanner] = None,
    ):
        self.tool_manager = tool_manager
        self.context = context or ContextManager()
        self.max_iterations = max_iterations
        self.max_retries_per_step = max_retries_per_step
        self.planner = planner or TaskPlanner()
        
        # Checkpoint state
        self.checkpoints: List[Checkpoint] = []
        self.current_plan: Optional[Plan] = None
        self.intermediate_results: Dict[str, Any] = {}
        
    async def run(self, user_input: str) -> str:
        """
        Main entry point for the agent loop.
        
        Args:
            user_input: The user's input message
            
        Returns:
            The agent's final response string
        """
        self.context.add_message("user", user_input)
        
        iteration = 0
        while iteration < self.max_iterations:
            # Think - decide what to do
            thought = await self._think(self.context.get_messages())
            
            if thought.action == ActionType.RESPOND:
                # We're done, return the response
                self.context.add_message("assistant", thought.content)
                return thought.content
            
            elif thought.action == ActionType.CALL_TOOL:
                # Execute the tool
                result = await self._act(thought.tool_call)
                
                # Observe - add result to context
                await self._observe(result)
                
            elif thought.action == ActionType.ASK_CLARIFICATION:
                self.context.add_message("assistant", thought.content)
                return thought.content
            
            iteration += 1
        
        # Max iterations reached, return a fallback response
        fallback = "I've reached the maximum number of iterations. Let me provide what I have so far."
        self.context.add_message("assistant", fallback)
        return fallback

    async def _think(self, messages: list[dict]) -> Thought:
        """
        Analyze messages and decide next action.
        
        Uses LLM to decide whether to:
        - Call a tool
        - Respond directly
        - Ask for clarification
        
        Args:
            messages: List of message dicts with 'role' and 'content'
            
        Returns:
            Thought object containing the decision
        """
        # TODO: Replace with actual LLM call
        # For now, use simple mock logic
        return await self._mock_llm_think(messages)

    async def _mock_llm_think(self, messages: list[dict]) -> Thought:
        """
        Mock LLM decision logic for development.
        
        TODO: Replace with actual OpenAI API call:
        
        ```python
        response = await openai.ChatCompletion.acreate(
            model="gpt-4",
            messages=messages,
            functions=self.tool_manager.get_schemas()
        )
        ```
        """
        # Simple heuristic: if last message contains "execute", call tool
        last_message = messages[-1]["content"] if messages else ""
        
        if "execute" in last_message.lower() or "run" in last_message.lower():
            # Try to extract a tool name
            tool_name = self._extract_tool_name(last_message)
            if tool_name:
                return Thought(
                    action=ActionType.CALL_TOOL,
                    tool_call=ToolCall(name=tool_name, arguments={})
                )
        
        return Thought(
            action=ActionType.RESPOND,
            content=f"I understand you said: {last_message}. How would you like me to proceed?"
        )

    def _extract_tool_name(self, text: str) -> Optional[str]:
        """Simple extraction of tool name from text"""
        available = self.tool_manager.list_tools()
        for tool in available:
            if tool.lower() in text.lower():
                return tool
        return None

    async def _act(self, tool_call: ToolCall) -> str:
        """
        Execute a tool call.
        
        Args:
            tool_call: The tool call to execute
            
        Returns:
            The tool execution result as string
        """
        try:
            result = await self.tool_manager.call_tool(
                tool_call.name,
                tool_call.arguments
            )
            return str(result)
        except Exception as e:
            return f"Error executing {tool_call.name}: {str(e)}"

    async def _observe(self, result: str) -> None:
        """
        Add tool execution result to context for next iteration.
        
        Args:
            result: The result from tool execution
        """
        self.context.add_message("system", f"Tool result: {result}")

    # === Checkpoint & Retry Methods ===

    def _save_checkpoint(self, step_index: int) -> None:
        """Save current state as checkpoint"""
        checkpoint = Checkpoint(
            step_index=step_index,
            messages_snapshot=copy.deepcopy(self.context.get_messages()),
            intermediate_results=copy.deepcopy(self.intermediate_results),
            plan_snapshot=copy.deepcopy(self.current_plan),
        )
        self.checkpoints.append(checkpoint)

    def _restore_checkpoint(self, step_index: int) -> bool:
        """Restore state from checkpoint"""
        for cp in reversed(self.checkpoints):
            if cp.step_index == step_index:
                self.context.restore_messages(copy.deepcopy(cp.messages_snapshot))
                self.intermediate_results = copy.deepcopy(cp.intermediate_results)
                if cp.plan_snapshot:
                    self.current_plan = cp.plan_snapshot
                return True
        return False

    async def _execute_goal(self, goal: SubGoal) -> bool:
        """Execute a single sub-goal"""
        goal.status = "in_progress"
        self.context.add_message("user", goal.description)
        
        # Think
        thought = await self._think(self.context.get_messages())
        
        if thought.action == ActionType.RESPOND:
            goal.status = "completed"
            return True
        
        if thought.action == ActionType.CALL_TOOL:
            result = await self._act(thought.tool_call)
            await self._observe(result)
            
            # Check if successful
            if "Error" not in result:
                goal.status = "completed"
                return True
            else:
                goal.error = result
                goal.status = "failed"
                goal.retry_count += 1
                return False
        
        return False

    async def _handle_failure(self, goal: SubGoal, step_index: int) -> bool:
        """Handle goal failure with retry or replanning"""
        if goal.retry_count < self.max_retries_per_step:
            # Retry the same goal
            goal.retry_count += 1
            self._restore_checkpoint(step_index)
            return await self._execute_goal(goal)
        else:
            # Try replanning
            new_goals = await self.planner.replan(goal, self.current_plan)
            self.current_plan.goals.extend(new_goals)
            self._restore_checkpoint(step_index)
            return await self._execute_goal(new_goals[0])

    async def run_planned(self, user_input: str) -> str:
        """
        Run with task decomposition and checkpoint/retry.
        
        Args:
            user_input: The user's task
            
        Returns:
            Final response string
        """
        # 1. Decompose into sub-goals
        self.current_plan = await self.planner.decompose(user_input)
        
        # 2. Execute each goal
        for i, goal in enumerate(self.current_plan.goals):
            self._save_checkpoint(i)
            success = await self._execute_goal(goal)
            
            if not success:
                if not await self._handle_failure(goal, i):
                    return f"Task failed at step {i+1}: {goal.description}"
            
            goal.status = "completed"
        
        return self._finalize_response()

    def _finalize_response(self) -> str:
        """Generate final response from results"""
        results = [g.description for g in self.current_plan.goals 
                  if g.status == "completed"]
        return f"Completed {len(results)} steps: {', '.join(results[-3:])}"

    async def run_stream(
        self, user_input: str
    ) -> AsyncGenerator[Dict[str, Any], None]:
        """
        Run agent with streaming events for UI visualization.
        
        Yields events:
        - {"type": "think", "content": str}
        - {"type": "tool_call", "tool_name": str, "params": dict}
        - {"type": "tool_result", "result": str}
        - {"type": "response", "content": str}
        """
        self.context.add_message("user", user_input)
        yield {"type": "think", "content": f"Processing: {user_input}"}

        iteration = 0
        while iteration < self.max_iterations:
            thought = await self._think(self.context.get_messages())
            
            if thought.action == ActionType.RESPOND:
                self.context.add_message("assistant", thought.content)
                yield {"type": "response", "content": thought.content}
                return

            elif thought.action == ActionType.CALL_TOOL:
                yield {
                    "type": "tool_call",
                    "tool_name": thought.tool_call.name,
                    "params": thought.tool_call.arguments,
                }
                
                result = await self._act(thought.tool_call)
                yield {"type": "tool_result", "result": result}
                
                await self._observe(result)

            iteration += 1

        yield {
            "type": "response",
            "content": "Max iterations reached",
        }
