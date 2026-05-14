# MemFlow 部门代码任务表（可直接发 OpenCode）

版本：v1.0
日期：2026-04-19
适用范围：MemFlow 现有代码仓（executor / learning-engine / web-ui / cli）
目标：把“统一智能操作系统 + 自我学习引擎”方案转成可执行、可验收、可追踪的代码任务

## 一、适配结论（已对齐你给的文档）

本任务表综合了以下文档要求并映射到 MemFlow：
1. 统一接入层、任务中心、Agent Runtime、Skill、Workflow、Memory、Learning、后台、监控审计
2. WBS 研发落地拆解、API 清单、页面清单、测试与上线要求
3. 自我学习引擎闭环：输入 -> 预处理 -> 学习 -> 分流 -> 规划 -> 执行 -> 验证 -> 夜间复盘
4. OpenCode 执行规范：每项任务必须带变更文件、运行命令、验证结果、证据路径

## 二、部门与代码边界

1. 平台架构组：模块边界、任务模型、跨模块协议
2. 后端执行组：Rust executor、compiler、http_server、workflow_registry
3. 学习引擎组：learning-engine、策略更新、夜间复盘、验证器
4. 数据与存储组：SQLite/PostgreSQL schema、迁移、索引、审计数据
5. 前端产品组：web-ui 管理台与工作流可视化
6. 测试与质量组：E2E、回归、接口与性能测试
7. 运维与可观测组：部署、监控、告警、回滚

## 三、P0 任务（必须先做，2-3 周）

| ID | 优先级 | 部门 | 代码范围 | 开发任务 | 交付物 | 验收标准 | 估时 |
|---|---|---|---|---|---|---|---|
| MF-P0-01 | P0 | 平台架构组 | executor + learning-engine | 统一任务对象与状态机（created/running/blocked/review/done/failed） | TaskState 统一定义 + 状态迁移校验器 | 任一任务状态变化可追踪且非法迁移被拒绝 | 1.5d |
| MF-P0-02 | P0 | 后端执行组 | executor/src/http_server.rs | 把 optimize/summarize/enhance-nl-workflow 从示例返回升级为真实计算链路 | 三个端点真实实现 | 端点返回真实学习数据，不再固定写死 | 2d |
| MF-P0-03 | P0 | 学习引擎组 | learning-engine | 建立 summarize 聚合器（成功率、平均耗时、失败归因、建议） | summarize service + 单测 | 输入样例日志后可产出结构化 insights | 2d |
| MF-P0-04 | P0 | 学习引擎组 | learning-engine | 建立参数优化器 v1（基于历史执行） | param_optimizer v1 + 回放脚本 | optimize 返回可解释参数建议（影响等级+理由） | 2d |
| MF-P0-05 | P0 | 数据与存储组 | migrations + db layer | 增加执行学习核心表：tasks、task_events、learning_units、learning_results、learning_actions | SQL 迁移 + DAO | 本地迁移通过，增删改查通过 | 1.5d |
| MF-P0-06 | P0 | 后端执行组 | executor | 对接任务证据链（request_id、owner、checkpoint、evidence） | API + 存储写入 | 每次执行可回放关键证据链 | 1.5d |
| MF-P0-07 | P0 | 前端产品组 | web-ui | 打通 AutoTuner/LearningReport/NLWizard 到真实后端 | 前端联调提交 | 三个面板均显示真实返回数据 | 1d |
| MF-P0-08 | P0 | 测试与质量组 | tests + web-ui e2e | 建立端到端主链路测试（创建 -> 执行 -> 学习 -> 优化） | E2E 用例 3 条 | 3 条链路稳定通过，可重复执行 | 1.5d |
| MF-P0-09 | P0 | 运维与可观测组 | docker-compose + metrics | 增加关键指标：任务成功率、平均耗时、优化命中率、学习产出数 | Prometheus 指标 + Grafana 面板 | 面板可看到实时趋势，告警规则可触发 | 1d |

## 四、P1 任务（平台能力完善，3-5 周）

| ID | 优先级 | 部门 | 代码范围 | 开发任务 | 交付物 | 验收标准 | 估时 |
|---|---|---|---|---|---|---|---|
| MF-P1-01 | P1 | 后端执行组 | executor + agent-service | 统一接入层规范（API/Webhook/消息渠道统一对象） | inbound adapter 层 | 不同入口进入后字段结构一致 | 2d |
| MF-P1-02 | P1 | 后端执行组 | executor | 任务中心 API（创建、更新、查询、阻塞、归档） | /tasks 系列接口 | 全生命周期可追踪 | 2d |
| MF-P1-03 | P1 | 学习引擎组 | learning-engine | Learning Loop v1（成功模板、失败归因、重复 blocker procedure） | learning loop 作业 | 至少 1 条真实学习闭环跑通 | 2.5d |
| MF-P1-04 | P1 | 学习引擎组 | learning-engine + scheduler | 夜间复盘任务 nightly review | nightly job + 报告 | 每天自动生成学习报告并落库 | 1.5d |
| MF-P1-05 | P1 | 数据与存储组 | memory-hub + db | 记忆四分层模型：实体/事件/流程/偏好 | schema + 检索接口 | 新任务可命中历史记忆 | 2d |
| MF-P1-06 | P1 | 前端产品组 | web-ui | 管理台页面补齐：任务看板、任务详情、日志回放、系统健康 | 页面与路由 | 管理员可在 UI 完成核心诊断 | 2.5d |
| MF-P1-07 | P1 | 前端产品组 | web-ui | Workflow 管理页与运行详情页 | 列表页 + 详情页 | 至少 3 条 workflow 可查看运行轨迹 | 2d |
| MF-P1-08 | P1 | 测试与质量组 | tests | 接口测试与回归测试（权限、幂等、超时） | 测试报告 | 覆盖核心 API 并门禁化 | 2d |
| MF-P1-09 | P1 | 运维与可观测组 | deploy + scripts | 灰度发布与回滚脚本标准化 | deploy playbook | 20 分钟内可回滚到上一版本 | 1.5d |

## 五、P2 任务（产品化与扩展，4-6 周）

| ID | 优先级 | 部门 | 代码范围 | 开发任务 | 交付物 | 验收标准 | 估时 |
|---|---|---|---|---|---|---|---|
| MF-P2-01 | P2 | 学习引擎组 | learning-engine | Skill -> Workflow 升级判定器（高频、稳定、可验收） | upgrade evaluator | 自动识别可流程化技能 | 2d |
| MF-P2-02 | P2 | 后端执行组 | workflow engine | Workflow 版本治理（发布、回滚、差异对比） | version API + 审计 | 任意版本可回滚可追踪 | 2d |
| MF-P2-03 | P2 | 前端产品组 | web-ui | 规则与策略可视化（学习结果、规则命中、策略变更） | 策略看板 | 运营可读可筛选可导出 | 2d |
| MF-P2-04 | P2 | 数据与存储组 | analytics | 成本与收益分析（人力节省、自动化命中） | impact analyzer 报表 | 管理层可看 ROI 变化趋势 | 2d |
| MF-P2-05 | P2 | 测试与质量组 | benchmark | 性能压测（并发任务创建、检索、workflow 执行） | 压测报告 | 达到目标阈值并给出瓶颈建议 | 1.5d |
| MF-P2-06 | P2 | 运维与可观测组 | observability | 业务告警模板（失败率、时延、学习异常） | 告警策略集 | 告警误报率可控，恢复路径明确 | 1d |

## 六、OpenCode 执行格式（强制）

每个任务卡必须按以下结构回复：
1. 任务编号
2. 当前状态
3. 已完成动作
4. 变更文件清单
5. 运行命令
6. 验证方式
7. 验证结果
8. 证据路径
9. 剩余风险
10. 下一步

不接受仅回复“已完成”。必须有代码、命令、结果、证据。

## 七、建议执行顺序

1. 先完成 P0（尤其是 MF-P0-02/03/04/05/08）
2. 再进入 P1（任务中心 + Learning Loop + 管理台）
3. 最后做 P2（自动升级与产品化优化）

## 八、首批验收门槛（给老板看的完成定义）

1. 任何输入都能转为标准任务并追踪状态
2. 工作流执行结果可写入记忆并在后续命中
3. optimize/summarize/enhance 三接口提供真实学习结果
4. 至少 3 条业务流程端到端跑通
5. 后台可查看任务、流程、日志、告警与健康状态
6. 出现异常可定位、可告警、可回滚
