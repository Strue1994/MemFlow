"""MCP Client - Connect to Model Context Protocol servers"""

import asyncio
import json
from typing import Any, Optional
from dataclasses import dataclass, field
from enum import Enum

import httpx


class MCPTransport(Enum):
    """MCP transport type"""
    STDIO = "stdio"
    HTTP = "http"


@dataclass
class MCPServerInfo:
    """Information about an MCP server"""
    name: str
    version: str
    description: str = ""


@dataclass
class MCPTool:
    """Tool definition from MCP server"""
    name: str
    description: str
    input_schema: dict = field(default_factory=dict)


class MCPClient:
    """
    Client for connecting to MCP (Model Context Protocol) servers.
    
    MCP is a protocol for communicating with external tools.
    This client can connect to MCP servers via STDIO or HTTP.
    
    Example:
        ```python
        client = MCPClient("http://localhost:3000")
        await client.connect()
        
        # Register tools with tool manager
        for tool in client.list_tools():
            manager.register_tool(
                tool.name,
                tool.input_schema,
                lambda args: client.call_tool(tool.name, args)
            )
        ```
    
    TODO: Implement actual MCP protocol using `mcp` SDK
    """

    def __init__(
        self,
        server_url: str = "http://localhost:3000",
        transport: MCPTransport = MCPTransport.HTTP,
    ):
        self.server_url = server_url
        self.transport = transport
        self._connected = False
        self._server_info: Optional[MCPServerInfo] = None
        self._tools: list[MCPTool] = []
        self._client: Optional[httpx.AsyncClient] = None

    async def connect(self) -> bool:
        """
        Connect to MCP server.
        
        Returns:
            True if connection successful
        """
        try:
            self._client = httpx.AsyncClient(timeout=30.0)
            
            # Initialize connection
            response = await self._client.get(f"{self.server_url}/health")
            if response.status_code == 200:
                self._connected = True
                self._server_info = MCPServerInfo(
                    name="memflow-mcp",
                    version="1.0.0",
                    description="Connected"
                )
                await self._fetch_tools()
                return True
                
        except Exception as e:
            print(f"[MCP] Connection failed: {e}")
            
        self._connected = False
        return False

    async def disconnect(self) -> None:
        """Disconnect from MCP server"""
        if self._client:
            await self._client.aclose()
        self._connected = False

    async def call_tool(self, tool_name: str, arguments: dict) -> str:
        """
        Call a tool on the MCP server.
        
        Args:
            tool_name: Name of the tool to call
            arguments: Tool arguments
            
        Returns:
            Tool result as string
        """
        if not self._connected:
            await self.connect()
            
        try:
            response = await self._client.post(
                f"{self.server_url}/tools/{tool_name}",
                json=arguments
            )
            return response.json()
        except Exception as e:
            return f"Error: {e}"

    async def list_tools(self) -> list[MCPTool]:
        """
        List available tools from the server.
        
        Returns:
            List of MCPTool definitions
        """
        if not self._connected:
            await self.connect()
            
        return self._tools

    async def _fetch_tools(self) -> None:
        """Fetch tool definitions from server"""
        if not self._client:
            return
            
        try:
            response = await self._client.get(f"{self.server_url}/tools")
            if response.status_code == 200:
                data = response.json()
                self._tools = [
                    MCPTool(
                        name=t["name"],
                        description=t.get("description", ""),
                        input_schema=t.get("inputSchema", {}),
                    )
                    for t in data.get("tools", [])
                ]
        except Exception as e:
            print(f"[MCP] Failed to fetch tools: {e}")

    def is_connected(self) -> bool:
        """Check if connected to server"""
        return self._connected

    def get_server_info(self) -> Optional[MCPServerInfo]:
        """Get server information"""
        return self._server_info


async def create_filesystem_server() -> dict:
    """
    Create a simple filesystem MCP server definition.
    
    This provides a reference for what a tool server should expose.
    """
    return {
        "name": "filesystem",
        "version": "1.0.0",
        "tools": [
            {
                "name": "read_file",
                "description": "Read contents of a file",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path to read"}
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "write_file", 
                "description": "Write content to a file",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "File path to write"},
                        "content": {"type": "string", "description": "Content to write"}
                    },
                    "required": ["path", "content"]
                }
            },
            {
                "name": "list_directory",
                "description": "List files in a directory",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Directory path"}
                    }
                }
            }
        ]
    }