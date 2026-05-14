"""MemFlow MCP Server implementation"""

import asyncio
import os
from typing import Any

import httpx

# Try importing MCP SDK
try:
    from mcp.server import Server
    from mcp.server.models import InitializationOptions
    from mcp.server.stdio import stdio_server
    import mcp.types as types
    MCP_AVAILABLE = True
except ImportError:
    MCP_AVAILABLE = False
    Server = None

# Configuration
MEMFLOW_API_BASE = os.getenv("MEMFLOW_API_BASE", "http://localhost:8000")


async def handle_list_tools() -> list:
    """Return list of available MemFlow tools."""
    if not MCP_AVAILABLE:
        return []
    
    return [
        types.Tool(
            name="execute_workflow",
            description="Execute a pre-defined MemFlow workflow by ID",
            inputSchema={
                "type": "object",
                "properties": {
                    "workflow_id": {
                        "type": "string",
                        "description": "The workflow ID to execute",
                    },
                    "params": {
                        "type": "object",
                        "description": "Input parameters for the workflow",
                    },
                },
                "required": ["workflow_id"],
            },
        ),
        types.Tool(
            name="search_memory",
            description="Search MemFlow's long-term memory for relevant information",
            inputSchema={
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query",
                    },
                    "limit": {
                        "type": "integer",
                        "default": 5,
                        "description": "Number of results",
                    },
                },
                "required": ["query"],
            },
        ),
        types.Tool(
            name="run_agent",
            description="Run MemFlow agent with a task description",
            inputSchema={
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "Task description for the agent",
                    },
                },
                "required": ["task"],
            },
        ),
    ]


async def handle_call_tool(name: str, arguments: dict) -> list:
    """Handle tool call requests."""
    if not MCP_AVAILABLE:
        return [types.TextContent(type="text", text="MCP SDK not available")]

    import json

    async with httpx.AsyncClient(timeout=60.0) as client:
        try:
            if name == "execute_workflow":
                resp = await client.post(
                    f"{MEMFLOW_API_BASE}/workflows/{arguments.get('workflow_id')}/execute",
                    json=arguments.get("params", {}),
                )
                result = resp.json() if resp.status_code == 200 else {"error": resp.text}
                return [types.TextContent(type="text", text=json.dumps(result, indent=2))]

            elif name == "search_memory":
                resp = await client.post(
                    f"{MEMFLOW_API_BASE}/memory/search",
                    json={
                        "query": arguments.get("query"),
                        "limit": arguments.get("limit", 5),
                    },
                )
                result = resp.json() if resp.status_code == 200 else {"error": resp.text}
                return [types.TextContent(type="text", text=json.dumps(result, indent=2))]

            elif name == "run_agent":
                resp = await client.post(
                    f"{MEMFLOW_API_BASE}/agent/run",
                    json={"task": arguments.get("task")},
                )
                result = resp.json() if resp.status_code == 200 else {"error": resp.text}
                return [types.TextContent(type="text", text=json.dumps(result, indent=2))]

            else:
                return [types.TextContent(type="text", text=f"Unknown tool: {name}")]

        except httpx.ConnectError:
            return [
                types.TextContent(
                    type="text",
                    text=f"Cannot connect to MemFlow API at {MEMFLOW_API_BASE}. "
                    "Make sure MemFlow is running.",
                )
            ]
        except Exception as e:
            return [types.TextContent(type="text", text=f"Error: {str(e)}")]


async def main() -> None:
    """Main entry point for the MCP server."""
    if not MCP_AVAILABLE:
        print("Error: MCP SDK not installed. Run: pip install mcp")
        return

    server = Server("memflow-mcp")

    @server.list_tools()
    async def list_tools() -> list:
        return await handle_list_tools()

    @server.call_tool()
    async def call_tool(name: str, arguments: dict) -> list:
        return await handle_call_tool(name, arguments)

    async with stdio_server() as streams:
        await server.run(
            streams[0],
            streams[1],
            InitializationOptions(
                server_name="memflow-mcp",
                server_version="0.1.0",
                capabilities=server.get_capabilities(),
            ),
        )


if __name__ == "__main__":
    asyncio.run(main())