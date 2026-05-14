"""MemFlow Configuration"""

import os
from dataclasses import dataclass


@dataclass
class Config:
    """Global configuration for MemFlow"""
    
    # API Keys
    openai_api_key: str = os.getenv("OPENAI_API_KEY", "")
    anthropic_api_key: str = os.getenv("ANTHROPIC_API_KEY", "")
    e2b_api_key: str = os.getenv("E2B_API_KEY", "")
    
    # Model settings
    default_model: str = os.getenv("MEMFLOW_MODEL", "gpt-4")
    temperature: float = 0.7
    
    # Agent settings
    max_iterations: int = int(os.getenv("MEMFLOW_MAX_ITERATIONS", "10"))
    max_tokens_context: int = int(os.getenv("MEMFLOW_MAX_TOKENS", "6000"))
    
    # MCP settings
    mcp_server_url: str = os.getenv("MCP_SERVER_URL", "http://localhost:3000")
    mcp_timeout: int = 30
    
    # Sandbox settings
    sandbox_backend: str = os.getenv("SANDBOX_BACKEND", "e2b")
    
    # System prompts
    system_prompt: str = """You are MemFlow, an AI assistant that can help with various tasks.

You have access to tools that can:
- Execute Python code in a sandbox
- Search the web
- Read/write files
- And more...

When asked to do something, decide whether to:
1. Use an available tool
2. Ask for clarification
3. Respond directly

Be helpful, concise, and accurate."""


# Global config instance
config = Config()


def get_config() -> Config:
    """Get global config instance"""
    return config