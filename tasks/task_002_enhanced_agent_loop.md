# 任务 002：增强 Agent 循环 - 添加 Checkpoint，重试与动态重规划

## 项目背景
当前 `core/agent_loop.py` 是线性执行，无容错能力。需升级为生产级循环。

## 详细要求

### 1. 新建 `core/task_planner.py`
```python
from typing import List
from pydantic import BaseModel

class SubGoal(BaseModel):
    description: str
    status: str = "pending"  # pending, in_progress, completed, failed

class TaskPlanner:
    def __init__(self, llm_client):
        self.llm = llm_client

    async def decompose(self, user_input: str) -> List[SubGoal]:
        """
        调用 LLM 将用户输入拆解为子目标列表。
        Prompt 示例：
        "将以下任务分解为3-5个可执行的步骤，每步一个短句。任务：{user_input}"
        返回格式：JSON 数组，如 ["步骤1", "步骤2"]
        """
        pass
```

### 2. 增强 `core/agent_loop.py`

**新增数据结构：**
```python
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional
import copy

@dataclass
class Checkpoint:
    step_index: int
    messages_snapshot: List[Dict[str, str]]
    intermediate_results: Dict[str, Any] = field(default_factory=dict)
```

**AgentLoop 类新增属性：**
- `max_retries_per_step: int = 2`
- `checkpoints: List[Checkpoint] = []`
- `current_plan: List[SubGoal] = []`

**新增/修改方法：**
```python
async def run(self, user_input: str) -> str:
    # 1. 规划
    self.current_plan = await self.planner.decompose(user_input)
    # 2. 逐步执行
    for i, goal in enumerate(self.current_plan):
        goal.status = "in_progress"
        # 保存检查点
        self._save_checkpoint(i)
        success = await self._execute_goal(goal)
        if not success:
            # 尝试重试或重规划
            if not await self._handle_failure(goal, i):
                return f"任务在步骤 {i+1} 失败: {goal.description}"
        goal.status = "completed"
    return self._finalize_response()
```

### 3. 测试用例
在 `tests/test_agent_loop.py` 中编写：
- `test_retry_on_tool_failure`: 模拟一个总是失败一次然后成功的工具。
- `test_replan_on_persistent_failure`: 模拟工具连续失败触发重规划。

## 验收标准
- 代码添加完整类型注解和文档字符串。
- 运行测试全部通过。
- 添加一个演示脚本 `examples/demo_resilience.py` 展示重试和回滚行为。