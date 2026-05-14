"""Task Planner - Decompose user input into actionable sub-goals"""

import json
from typing import List, Optional, Any
from pydantic import BaseModel


class SubGoal(BaseModel):
    """A single sub-goal in a decomposed task"""
    description: str
    status: str = "pending"  # pending, in_progress, completed, failed
    retry_count: int = 0
    error: Optional[str] = None


class Plan(BaseModel):
    """A complete plan with multiple sub-goals"""
    goals: List[SubGoal]
    original_input: str
    current_index: int = 0


class TaskPlanner:
    """
    Decomposes user input into executable sub-goals.
    
    Uses LLM to break down complex tasks into smaller,
    manageable steps that can be executed sequentially.
    
    Example:
        ```python
        planner = TaskPlanner(llm_client)
        plan = await planner.decompose("Write a web scraper")
        # Returns: [SubGoal("Set up project structure"), 
        #            SubGoal("Implement scraping logic"), ...]
        ```
    """

    def __init__(self, llm_client: Optional[Any] = None):
        self.llm = llm_client

    async def decompose(self, user_input: str) -> Plan:
        """
        Decompose user input into sub-goals.
        
        Args:
            user_input: The user's task description
            
        Returns:
            Plan object containing list of SubGoals
        """
        if self.llm:
            return await self._llm_decompose(user_input)
        return await self._rule_based_decompose(user_input)

    async def _llm_decompose(self, user_input: str) -> Plan:
        """
        Use LLM to decompose task into sub-goals.
        
        TODO: Replace with actual LLM call:
        
        Example prompt:

        `将以下任务分解为3-5个可执行的步骤，每步一个短句。\n`
        `任务：{user_input}\n\n`
        `返回JSON数组格式：["步骤1", "步骤2", ...]`

        Then:

        `response = await self.llm.chat(prompt)`
        `goals = [SubGoal(description=g) for g in json.loads(response)]`
        `return Plan(goals=goals, original_input=user_input)`
        """
        # Placeholder - use rule-based for now
        return await self._rule_based_decompose(user_input)

    async def _rule_based_decompose(self, user_input: str) -> Plan:
        """
        Simple rule-based decomposition for development.
        
        Splits input by common delimiters.
        """
        input_lower = user_input.lower()
        goals = []
        
        # Common task patterns
        if "write" in input_lower or "create" in input_lower or "build" in input_lower:
            goals.append(SubGoal(description="Plan and structure the solution"))
            goals.append(SubGoal(description="Implement the core functionality"))
            goals.append(SubGoal(description="Add tests and documentation"))
        elif "fix" in input_lower or "bug" in input_lower or "error" in input_lower:
            goals.append(SubGoal(description="Identify the root cause"))
            goals.append(SubGoal(description="Implement the fix"))
            goals.append(SubGoal(description="Verify the fix works"))
        elif "search" in input_lower or "find" in input_lower:
            goals.append(SubGoal(description="Define search criteria"))
            goals.append(SubGoal(description="Execute search"))
            goals.append(SubGoal(description="Present results"))
        else:
            # Default: just try to do it
            goals.append(SubGoal(description=user_input))
        
        return Plan(goals=goals, original_input=user_input)

    async def replan(self, failed_goal: SubGoal, original_plan: Plan) -> List[SubGoal]:
        """
        Generate a new plan when a goal fails.
        
        Args:
            failed_goal: The goal that failed
            original_plan: The original plan
            
        Returns:
            New list of SubGoals to try
        """
        # Simple retry with modified approach
        new_goals = [
            SubGoal(description=f"Retry: {failed_goal.description}"),
            SubGoal(description="Try alternative approach"),
        ]
        return new_goals


class MockLLMClient:
    """Mock LLM client for development/testing"""
    
    async def chat(self, prompt: str) -> str:
        """Return mock response"""
        return '["Step 1", "Step 2", "Step 3"]'
