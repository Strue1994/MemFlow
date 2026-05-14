# MemFlow

MemFlow Stage 1 now runs through a workflow-first task console. The primary user path starts in `web-ui/`, routes through `agent-service/`, and executes through the Rust `executor/`. The older Python loop in `main.py` and `core/agent_loop.py` is still present for experimentation, but it is not the main product path.

## Stage 1 Product Path

The Stage 1 flow is:

1. A user submits a natural-language task in the task console.
2. MemFlow determines whether the request is:
   - a repeatable workflow task, or
   - a one-off task.
3. The runtime then routes the task to one of three outcomes:
   - an existing workflow,
   - a generated workflow,
   - or an agent-driven execution path.
4. MemFlow executes the route and explains the result back in the console.

This is the intended product path for current development and verification work.

## Runtime Roles

- `executor/` = workflow execution kernel
- `agent-service/` = primary task-routing entrypoint
- `web-ui/` = workflow-first task console
- `main.py` / `core/agent_loop.py` = experimental Python loop, not the primary Stage 1 runtime

## Architecture Overview

At a high level, the repository is organized into four major layers:

1. **Workflow-first product runtime**
   - `web-ui/` provides the task console and Stage 1 user flow.
   - `agent-service/` accepts task requests, decides the execution path, and bridges the UI to the executor.
   - `executor/` runs workflows, persistence, plugins, concurrency, and learning-related modules.

2. **Python orchestration layer (experimental)**
   - `main.py` is the Python entry point for the local interactive agent.
   - `core/agent_loop.py` contains the main think-act-observe loop, plus checkpoint/retry scaffolding.
   - `core/tool_manager.py` owns tool registration, schema validation, and execution dispatch.
   - `core/task_planner.py` handles simple task decomposition and replanning.
   - `core/sandbox.py` abstracts code execution backends, with E2B as the primary implementation.
   - `utils/context_manager.py` manages conversation history and token-aware compression.
   - `config.py` centralizes runtime settings and environment-backed configuration.

3. **Operational tooling**
   - `ui/`, Docker files, local start/stop scripts, dashboards, and deployment assets support development and operations.

## Python Module Responsibilities

### `main.py`
Creates the default `ToolManager`, `ContextManager`, and `Sandbox`, registers a sandbox execution tool, and starts either:
- an interactive CLI loop, or
- a single-message execution mode.

### `core/agent_loop.py`
Implements the core agent loop with:
- response/tool/clarification action selection,
- tool execution and observation,
- lightweight checkpoint snapshots,
- retry scaffolding,
- task planner integration.

Important current limitation: the actual LLM reasoning path is still mocked by `_mock_llm_think()`.

### `core/tool_manager.py`
Provides a registry for tools and their schemas. It:
- registers callable tools,
- validates required arguments,
- supports async and sync execution,
- exposes schemas for future function-calling integration.

### `core/task_planner.py`
Provides:
- `SubGoal`
- `Plan`
- `TaskPlanner`

The current decomposition strategy is rule-based unless an external LLM client is injected.

### `core/sandbox.py`
Wraps isolated execution. Today it primarily supports:
- E2B-backed Python execution,
- limited JavaScript execution through E2B.

Local execution is explicitly not implemented for safety.

### `utils/context_manager.py`
Tracks conversation messages and supports token-aware compression. It currently uses:
- `tiktoken` for counting,
- heuristic truncation/compression instead of full summarization.

### `config.py`
Defines a dataclass-backed configuration object with environment variables for:
- API keys,
- model defaults,
- context size,
- sandbox backend,
- MCP server settings.

## Environment

Copy `.env.example` to `.env` and set at minimum:

- `EXECUTOR_API_KEY`: shared between `executor` and `agent-service`
- `OPENAI_API_KEY`: required for the `/chat` endpoint and any real LLM-backed functionality
- `E2B_API_KEY`: required if you want Python sandbox execution via E2B

## Installation

### Python dependencies

```bash
pip install -r requirements.txt
```

### Main Python requirements
- `openai`
- `tiktoken`
- `pydantic`
- `httpx`
- `apscheduler`
- `aiosqlite`
- `gradio`
- `python-dotenv`
- `aiofiles`
- `pytest`

### Local development prerequisites

- [Rust](https://rustup.rs/) with `stable-x86_64-pc-windows-msvc`
- [Node.js](https://nodejs.org/) 18+
- PowerShell 5.1+

## Running the Project

### Full stack via Docker

```bash
docker-compose up --build
```

### Local launcher (recommended on Windows)

```powershell
.\scripts\dev-local.ps1
```

This is the current Stage 1 local startup path on Windows. The launcher builds and starts the agent service, starts or reuses the executor, serves the Stage 1 frontend, and waits for readiness of:
- Executor (`8082`)
- Agent service (`3300`)
- Frontend (`5273`)

### Stop local services

```powershell
.\scripts\stop-local.ps1
```

### Python CLI entry

```bash
python main.py
```

### Single-message mode

```bash
python main.py --single "run python_exec"
```

## Start Components Individually

```powershell
# Executor
$env:RUSTUP_TOOLCHAIN="stable-x86_64-pc-windows-msvc"
cargo run --package executor -- serve --addr 127.0.0.1:8082

# Agent service
cd agent-service
npm run build
$env:PORT=3300
$env:EXECUTOR_URL="http://127.0.0.1:8082"
node dist/index.js

# Frontend
cd web-ui
$env:MEMFLOW_WEB_PORT=5273
npm run build
node ..\scripts\serve-web-ui.js
```

## Services

- Executor: `http://localhost:8080`
- Agent service: `http://localhost:3000`
- Frontend: `http://localhost:80`

The agent service forwards workflow compile and execute requests to the executor using `X-API-Key` authentication.

## Testing and Checks

### Python

```bash
pytest
```

### Rust workspace

```bash
cargo test --workspace
```

### Agent service

```bash
cd agent-service && npm test
```

## Known Limitations

- The Python `AgentLoop` still uses a mock reasoning implementation instead of a real model-backed planning loop.
- `TaskPlanner` currently falls back to rule-based decomposition for most cases.
- The local sandbox backend is intentionally not implemented; E2B is required for safe code execution.
- `ContextManager` uses heuristic compression rather than model-generated summarization.
- Some Python requirements reflect intended future features more than currently exercised production paths.

## Troubleshooting

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| Executor does not start in time | First Rust build still running or port `8082` is busy | Check `.memflow-runtime\logs\executor.log` and make sure `8082` is free |
| Agent service does not start in time | Node dependencies are missing or port `3300` is busy | Run `npm install` in `agent-service` and re-check port usage |
| Frontend is blank or does not load | Frontend dev server failed to start or backend proxy issue | Check `.memflow-runtime\logs\frontend.log` and confirm `http://127.0.0.1:3300` is reachable |
| `cargo` is not found | Rust is not installed or not on `PATH` | Install Rust and restart the terminal |
| Python code execution fails immediately | `E2B_API_KEY` missing or `e2b-code-interpreter` not installed | Configure the key and install the dependency |

## Repo Hygiene

- Build artifacts are ignored at the repo root via `.gitignore`
- Large local archives such as `tasks.zip` should stay out of version control
- Local runtime files such as `workflow_files/` and `*.db` are disposable state unless explicitly needed for debugging
