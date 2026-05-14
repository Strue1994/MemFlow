"""Tool Manager - Register and execute tools"""

import asyncio
from typing import Any, Callable, Optional
from dataclasses import dataclass, field
import json


@dataclass
class ToolSchema:
    """Schema definition for a tool"""
    name: str
    description: str
    parameters: dict = field(default_factory=dict)
    

class ToolManager:
    """
    Manages tool registration and execution.
    
    Tools can be registered with:
    - name: Unique identifier
    - schema: JSON schema describing parameters
    - func: Async callable to execute
    
    Example:
        ```python
        async def add(a: int, b: int) -> int:
            return a + b
            
        manager.register_tool(
            "add",
            {"a": int, "b": int},
            add
        )
        result = await manager.call_tool("add", {"a": 1, "b": 2})
        ```
    """

    def __init__(self):
        self._tools: dict[str, Callable] = {}
        self._schemas: dict[str, ToolSchema] = {}

    def register_tool(
        self,
        name: str,
        schema: dict,
        func: Callable,
    ) -> None:
        """
        Register a new tool.
        
        Args:
            name: Tool name identifier
            schema: JSON Schema for parameters
            func: Async function to call
        """
        if not isinstance(name, str) or not name.strip():
            raise ValueError("Tool name must be a non-empty string")
        if not isinstance(schema, dict):
            raise TypeError("Tool schema must be a dict")
        if not callable(func):
            raise TypeError("Tool function must be callable")
        self._tools[name] = func
        self._schemas[name] = ToolSchema(
            name=name,
            description=schema.get("description", ""),
            parameters=schema.get("parameters", {}),
        )

    async def call_tool(self, name: str, arguments: dict) -> Any:
        """
        Execute a registered tool.
        
        Args:
            name: Tool name to call
            arguments: Parameters to pass
            
        Returns:
            Tool execution result
            
        Raises:
            KeyError: If tool not found
        """
        if name not in self._tools:
            raise KeyError(f"Tool '{name}' not found")
        if not isinstance(arguments, dict):
            raise TypeError("Tool arguments must be a dict")
        
        func = self._tools[name]
        
        # Validate arguments against schema
        schema = self._schemas.get(name)
        if schema:
            self._validate_arguments(arguments, schema.parameters)
        
        # Execute
        if asyncio.iscoroutinefunction(func):
            return await func(**arguments)
        else:
            return func(**arguments)

    def _validate_arguments(self, arguments: dict, schema: dict) -> None:
        """Validate arguments against schema"""
        required = schema.get("required", [])
        for param in required:
            if param not in arguments:
                raise ValueError(f"Missing required parameter: {param}")

    def get_schemas(self) -> list[dict]:
        """Get all tool schemas for LLM function calling"""
        return [
            {
                "name": s.name,
                "description": s.description,
                "parameters": s.parameters,
            }
            for s in self._schemas.values()
        ]

    def list_tools(self) -> list[str]:
        """List all registered tool names"""
        return list(self._tools.keys())

    def get_tool(self, name: str) -> Optional[Callable]:
        """Get a tool by name"""
        return self._tools.get(name)


# Built-in tools
async def echo(message: str = "") -> str:
    """Echo back the input"""
    return f"Echo: {message}"


async def python_exec(code: str) -> str:
    """
    Execute Python code in sandbox.
    
    Uses Sandbox class with E2B backend for secure execution.
    """
    from .sandbox import Sandbox
    
    sandbox = Sandbox()
    result = await sandbox.execute_code(code, "python")
    
    if result.error:
        return f"Error: {result.error}\nStderr: {result.stderr}"
    
    output = result.stdout
    if result.stderr:
        output += f"\nStderr: {result.stderr}"
    
    return output or "[No output]"


async def search_web(query: str) -> str:
    """
    Search the web.
    
    TODO: Implement actual search
    """
    return f"[Mock] Would search for: {query}"


def create_default_manager() -> ToolManager:
    """Create manager with built-in tools"""
    manager = ToolManager()
    
    manager.register_tool(
        "echo",
        {
            "description": "Echo back a message",
            "parameters": {"message": {"type": "string"}}
        },
        echo
    )
    
    manager.register_tool(
        "python_exec",
        {
            "description": "Execute Python code in sandbox",
            "parameters": {"code": {"type": "string"}}
        },
        python_exec
    )
    
    manager.register_tool(
        "search_web",
        {
            "description": "Search the web",
            "parameters": {"query": {"type": "string"}}
        },
        search_web
    )
    
    return manager
