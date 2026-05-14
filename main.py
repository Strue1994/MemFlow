"""MemFlow - AI Agent with Tool Calling"""

import asyncio
import sys
from typing import Optional

from core.agent_loop import AgentLoop
from core.tool_manager import create_default_manager, ToolManager
from core.sandbox import Sandbox
from utils.context_manager import ContextManager
from config import config, get_config


async def run_interactive(agent: AgentLoop) -> None:
    """Run interactive command-line loop"""
    print("=" * 50)
    print("  MemFlow - AI Agent")
    print("=" * 50)
    print("Type 'quit' or 'exit' to end the session\n")
    
    while True:
        try:
            user_input = input("You: ").strip()
            
            if not user_input:
                continue
                
            if user_input.lower() in ["quit", "exit", "q"]:
                print("\nGoodbye!")
                break
            
            # Run agent
            response = await agent.run(user_input)
            print(f"\nMemFlow: {response}\n")
            
        except KeyboardInterrupt:
            print("\n\nGoodbye!")
            break
        except Exception as e:
            print(f"\nError: {e}\n")


async def run_single(agent: AgentLoop, message: str) -> str:
    """Run single message and return response"""
    return await agent.run(message)


async def main():
    """Main entry point"""
    # Parse arguments
    single_mode = "--single" in sys.argv
    message_idx = sys.argv.index("--single") + 1 if single_mode else None
    message = sys.argv[message_idx] if message_idx and len(sys.argv) > message_idx else None
    
    # Initialize components
    tool_manager = create_default_manager()
    context = ContextManager(
        model=config.default_model,
        max_tokens=config.max_tokens_context,
    )
    sandbox = Sandbox(backend=config.sandbox_backend)
    
    # Register sandbox tool
    async def sandbox_exec(code: str) -> str:
        return await sandbox.execute_code(code)
    
    tool_manager.register_tool(
        "sandbox_exec",
        {"description": "Execute Python code in sandbox", "parameters": {"code": {"type": "string"}}},
        sandbox_exec
    )
    
    # Create agent
    agent = AgentLoop(
        tool_manager=tool_manager,
        context=context,
        max_iterations=config.max_iterations,
    )
    
    # Run
    if single_mode and message:
        response = await run_single(agent, message)
        print(response)
    else:
        await run_interactive(agent)


if __name__ == "__main__":
    asyncio.run(main())