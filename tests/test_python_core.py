"""Minimal regression tests for MemFlow Python core utilities."""

import asyncio
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.agent_loop import AgentLoop
from core.task_planner import SubGoal
from core.tool_manager import ToolManager
from core.sandbox import Sandbox, ExecutionResult
from utils.context_manager import ContextManager


def test_context_restore_messages():
    ctx = ContextManager()
    ctx.add_message("user", "hello")
    snapshot = ctx.get_messages()
    ctx.clear()
    ctx.restore_messages(snapshot)
    assert ctx.get_messages() == snapshot


def test_sandbox_local_returns_execution_result():
    sandbox = Sandbox(backend="local")
    result = asyncio.run(sandbox.execute_code("print(1)"))
    assert isinstance(result, ExecutionResult)
    assert result.error is not None


def test_tool_manager_contract_checks():
    manager = ToolManager()

    async def echo(message: str = ""):
        return message

    manager.register_tool(
        "echo",
        {"description": "Echo", "parameters": {"required": ["message"]}},
        echo,
    )

    try:
        asyncio.run(manager.call_tool("echo", "not-a-dict"))
        assert False, "Expected TypeError for non-dict arguments"
    except TypeError:
        pass


def test_agent_loop_restore_checkpoint_uses_public_api():
    manager = ToolManager()
    ctx = ContextManager()
    loop = AgentLoop(tool_manager=manager, context=ctx)
    ctx.add_message("user", "first")
    loop._save_checkpoint(0)
    ctx.clear()
    restored = loop._restore_checkpoint(0)
    assert restored is True
    assert ctx.get_messages()[0]["content"] == "first"


if __name__ == "__main__":
    test_context_restore_messages()
    test_sandbox_local_returns_execution_result()
    test_tool_manager_contract_checks()
    test_agent_loop_restore_checkpoint_uses_public_api()
    print("MemFlow Python core tests passed.")
