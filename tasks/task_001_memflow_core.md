# 任务：为 MEMFLOW 项目搭建基础Agent循环与工具调用框架

## 项目背景
MEMFLOW是一个需要追赶HERMES AGENT能力的AI智能体项目。当前阶段的目标是搭建一个具备基础Agentic Loop、支持MCP工具协议、并拥有安全沙箱执行能力的Python框架。

## 技术栈要求
- 语言：Python 3.10+
- 框架：使用 `asyncio` 进行异步处理。
- LLM集成：支持OpenAI API标准，便于切换模型。
- 工具协议：初步集成MCP (Model Context Protocol) SDK。

## 具体任务

### 1. 项目结构初始化
创建以下目录和文件结构：
```
memflow/
├── core/
│   ├── __init__.py
│   ├── agent_loop.py      # 核心循环逻辑
│   ├── tool_manager.py  # 工具注册与调用管理
│   └── sandbox.py        # 沙箱执行接口
├── mcp/
│   ├── __init__.py
│   └── mcp_client.py     # MCP客户端，用于连接工具服务器
├── utils/
│   ├── __init__.py
│   └── context_manager.py  # 上下文管理与智能压缩
├── config.py             # 配置管理
├── main.py               # 入口文件
└── requirements.txt
```

### 2. 实现核心模块（接口与桩代码）

#### A. `core/agent_loop.py`
实现一个基础的 `AgentLoop` 类，包含以下核心方法：
- `async def run(self, user_input: str) -> str`：主循环入口。
- `async def _think(self, messages: list) -> dict`：调用LLM，决定下一步动作（调用工具或回复用户）。
- `async def _act(self, tool_call: dict) -> str`：执行工具调用。
- `async def _observe(self, result: str) -> None`：将结果加入上下文。

**要求**：为每个方法写好清晰的docstring和TODO注释。初期可以先使用一个简单的模拟逻辑（Mock LLM），但结构必须支持后续替换为真实模型。

#### B. `core/tool_manager.py`
实现 `ToolManager` 类：
- `def register_tool(self, name: str, schema: dict, func: callable)`：注册工具。
- `async def call_tool(self, name: str, arguments: dict) -> str`：执行指定工具。

#### C. `mcp/mcp_client.py`
实现一个 `MCPClient` 类：
- 能够连接到本地或远程的MCP服务器（使用标准MCP SDK）。
- 将MCP服务器提供的工具自动注册到 `ToolManager`。
- 提供一个基础的文件系统操作MCP服务器作为示例。

#### D. `core/sandbox.py`
实现 `Sandbox` 类：
- 提供一个 `async def execute_code(self, code: str, language: str = "python") -> str` 方法。
- **安全性要求**：必须使用`E2B` (https://e2b.dev) 或 `Docker` SDK 来创建隔离环境执行代码。先在代码中写好E2B的调用逻辑，如果无法配置则留出清晰的接口。

#### E. `utils/context_manager.py`
实现 `ContextManager` 类：
- `def add_message(self, role: str, content: str)`：添加消息。
- `def get_messages(self) -> list`：获取当前消息列表。
- `def maybe_compress(self) -> None`：**智能压缩**。当Token数量超过阈值（如6000）时，调用LLM生成对话摘要，并用摘要替换旧消息，确保关键信息不丢失。先实现一个基础的滑动窗口截断作为占位符。

### 3. 验收标准
- 代码可以直接运行（即使内部是mock逻辑），`python main.py` 能启动一个简单的命令行对话循环。
- 所有类和方法都有清晰的类型注解和docstring。
- 在关键位置（如沙箱执行、MCP连接）留下TODO注释，说明后续需要完善的部分。
- 生成 `requirements.txt` 文件，包含 `openai`, `mcp-sdk`, `e2b-code-interpreter`, `tiktoken`, `pydantic` 等基本依赖。

请开始生成上述文件的核心代码。