---
tracker:
  kind: plane
  project_name: "imperial-desk"
  project_identifier: "IMD"
  active_states:
    - Todo
    - In Progress
    - Review
  terminal_states:
    - Done
    - Cancelled
polling:
  interval_ms: 5000
agent:
  max_concurrent_agents: 4
  max_turns: 20
git:
  default_base_branch: main
---

# Imperial Desk Development Workflow

本仓库是 `imperial-desk` 的主实现仓库。
当前项目是一个 Rust workspace，目标是实现一个面向 web LLM 的 provider-agnostic CLI，当前优先支持 `deepseek-web`。

Plane 是任务系统事实来源。
GitHub 用于源码、提交和远程协作，但不替代 Plane 的任务记录。

## 默认工作姿态

1. 先读相关 Plane issue，再开始实现。
2. 先核对当前代码状态，不要只按 `docs/design.md` 假设功能存在。
3. 每次只完成一个明确能力范围，不要把设计讨论、实现和清理混成一个大 issue。
4. 变更行为前先确认基线，变更后做针对性验证。
5. 代码、文档、Plane 记录三者要同步。

## 状态约定

- `Todo`
  - 已确认要做，但尚未开始实现。
- `In Progress`
  - 正在分析、实现或验证。
- `Review`
  - 代码和验证已完成，等待人工复核或收尾处理。
- `Done`
  - 已完成并记录结果。
- `Cancelled`
  - 不再继续。

## Workpad 约定

每个活跃 issue 保持一个持续更新的评论，标题固定为：

```md
## Codex Workpad
```

该评论用于记录：

- 当前环境和工作目录
- 计划与范围
- 验收标准
- 验证命令与结果
- 风险、阻塞和后续项

Plane 项目内的内容统一使用中文：

- 项目说明
- issue 标题
- issue 描述
- workpad 评论
- review 结论
- 完成说明

代码符号、crate 名称、命令、路径等保留原文。

## 标准执行流程

### Step 0: 同步上下文

1. 读取当前 issue、状态和描述。
2. 如果 issue 处于 `Todo`，开始工作前先切到 `In Progress`。
3. 查找或创建 `## Codex Workpad` 评论。
4. 将 issue 中的 `Scope`、`Acceptance Criteria`、`Validation` 同步到 workpad。
5. 记录当前基线：
   - 绝对路径
   - 当前分支
   - 关键命令结果

### Step 1: 先定范围再动手

1. 明确当前 issue 是哪一类能力：
   - CLI 表层
   - core contract
   - agent 协调器
   - browser backend
   - state 持久化
   - provider 实现
   - 文档和流程
2. 按能力拆分，不按文件名拆分。
3. 发现明显超出当前 issue 的工作时，新增 follow-up issue，而不是静默扩 scope。

### Step 2: 实现

1. 保持 crate 边界清晰：
   - `web-llm-cli` 负责参数解析和输出
   - `web-llm-core` 负责共享类型和 trait
   - `web-llm-agent` 负责 provider-agnostic 协调逻辑
   - `web-llm-browser` 负责浏览器抽象与后端
   - `web-llm-state` 负责本地状态
   - `web-llm-provider` 负责厂商实现与注册
2. 不要把 DeepSeek 站点细节泄漏到 `web-llm-agent` 或 `web-llm-core`。
3. 架构或流程变更时，同时更新相关文档。

### Step 3: 验证

优先运行能直接证明本次改动的验证。

Rust 变更的默认验证顺序：

1. `cargo fmt --check`
2. `cargo test`
3. 如果改动跨 crate 或共享 contract，再补充更广验证

如果未执行某项验证，必须在 workpad 和最终说明里写明原因。

### Step 4: 收尾

1. 将验证结果回写到 workpad。
2. 将 issue 切到 `Review` 或 `Done`，不要跳过记录。
3. 提交前确认本次变更范围与 issue 一致。
4. 合并或交付后，再开始下一个 issue。

## 文档优先规则

以下情况优先更新文档，再拆实现 issue：

- crate 边界变化
- provider capability 变化
- login 流程变化
- agent protocol 变化
- workflow 或协作规则变化

涉及架构决策时，至少同步这些文件：

- `docs/design.md`
- `WORKFLOW.md`
- `AGENTS.md`

## Issue 拆分规则

推荐按能力拆分 issue，而不是按目录拆分。

当前项目适合的拆分粒度：

1. workspace 与基础文档
2. core types 与 provider registry
3. Chromium browser backend
4. DeepSeek web provider 基础交互
5. CLI 命令面
6. 本地状态与 provider config
7. agent 协调器与工具执行层
8. login wizard 与二维码流程
9. DeepSeek API provider
10. 测试与诊断

每个实现 issue 至少包含：

- `Scope`
- `Acceptance Criteria`
- `Validation`

## Git 流程

本仓库默认使用 `main` 作为主分支。

1. 若仓库尚未初始化，先完成 `git init`、远端绑定和首次提交。
2. 若已有远端和分支，优先在与当前 issue 对应的分支上工作。
3. 每个 issue 至少对应一组清晰提交。
4. 不要把多个无关能力挤进同一次提交。

## 当前阶段提醒

实现状态必须以代码为准，而不是只以设计文档为准。

当前仓库已经具备一部分实现基础，因此后续 issue 需要区分：

- 已落地的能力补录为已完成 issue
- 仍是占位、骨架或未闭环的能力录入为 `Todo`

尤其要注意：

- `deepseek-web` 已有较多实现
- `web-llm-agent` 仍是极简版本，不等于完整工具协调器
- `deepseek-api` 目前仍是占位
- 测试覆盖几乎为空
