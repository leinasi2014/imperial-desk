---
tracker:
  kind: plane
  project_name: "Imperial Desk"
  project_identifier: "IMD"
  active_states:
    - Backlog
    - Todo
    - In Progress
    - Review
  terminal_states:
    - Done
    - Cancelled
polling:
  interval_ms: 5000
codex:
  backend: app-server
  server_mode: managed
  thread_strategy: hybrid
  command: codex
  model: "gpt-5.4"
  personality: pragmatic
  service_tier: flex
  reasoning_effort: xhigh
  approval_policy: never
  thread_sandbox: danger-full-access
roles:
  development_owner: "张居正"
  review_owners:
    - "海瑞"
    - "于谦"
review:
  required_approvals: 2
git:
  auto_commit: false
  auto_push: false
  auto_pr: false
  auto_land: false
metadata:
  generated_from: "plane-role-orchestration"
  generated_at: "2026-03-12T04:04:12"
  repo_root: "D:\\workspaces\\deepseek\\imperial-desk"
---

# Imperial Desk Development Workflow

本仓库是 `imperial-desk` 的主实现仓库。
当前项目是一个 Rust workspace，目标是实现一个面向 web LLM 的 provider-agnostic CLI，当前优先支持 `deepseek-web`。

Plane 是任务系统事实来源。
Git 用于源码、提交和本地协作，但不替代 Plane 的任务记录。

## 项目标识

- Plane 项目名：`Imperial Desk`
- Plane 项目标识：`IMD`
- 主设计文档：`docs/design.md`

当前工作流以本文件为标准，Plane 状态、assignee 路由、workpad 和记
录、以及本地 git 现实必须保持一致。

## 默认工作姿态

1. 先读相关 Plane issue，再开始实现。
2. 先核对当前代码状态，不要只按 `docs/design.md` 假设功能存在。
3. 每次只完成一个明确能力范围，不要把设计讨论、实现和清理混成一个大 issue。
4. 变更行为前先确认基线，变更后做针对性验证。
5. 代码、文档、Plane 记录三者要同步。

## 状态约定

- `Backlog`
  - 范围已经记录，但尚未准备进入当前实现轮次。
- `Todo`
  - 已确认要做，且已准备好进入下一次实现。
- `In Progress`
  - 正在由 `Development Owner` 和开发子代理执行实现或自验证。
- `Review`
  - 代码和验证已完成，当前由 `Project Reviewer` 和审查子代理执行独立审查。
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
- 角色交接、风险、阻塞和后续项

Plane 项目内的内容统一使用中文：

- 项目说明
- issue 标题
- issue 描述
- workpad 评论
- review 结论
- 完成说明

代码符号、crate 名称、命令、路径等保留原文。

## 角色与接管流程

本项目采用四层角色模型：

1. `Coordinator`
2. `Development Owner`
3. `Review Owner A`
4. `Review Owner B`

当前 Plane 角色标准：

- `Coordinator`
  - 当前对应 `徐阶`
  - 负责 issue 路由、状态推进、workpad 和交付就绪判断
- `Development Owner`
  - 当前对应 `张居正`
  - 负责 `In Progress` 阶段的 Plane assignee 身份
- `Review Owner A`
  - 当前对应 `海瑞`
  - 负责第一条审查链路
- `Review Owner B`
  - 当前对应 `于谦`
  - 负责第二条审查链路

执行层由子代理接管：

- 开发子代理接管 `Development Owner` 的具体实现工作
- 审查子代理 A 接管 `Review Owner A` 的具体审查工作
- 审查子代理 B 接管 `Review Owner B` 的具体审查工作

规则：

1. `Coordinator` 不是默认开发 assignee，也不是默认审查 assignee。
2. Plane 中不能依赖虚拟成员，因此角色名要与真实成员或待补齐成员标准一致。
3. 一旦某个 `Todo` issue 进入真实开发，必须指派给 `张居正` 并切到 `In Progress`。
4. 进入审查时，必须同时路由给 `海瑞` 和 `于谦`，并切到 `Review`。
5. 一个活跃开发 issue 默认只对应一个开发执行负责人，避免职责漂移。
6. 如果需要并行拆分，主 agent 可以创建多个子代理，但每个子代理必须有明确且不重叠的文件或模块边界。
7. Plane 的状态更新、workpad 维护和最终结项仍由主 agent 承担，不由子代理直接承担。

## 标准执行流程

### Step 0: 同步上下文

1. 读取当前 issue、状态和描述。
2. 查找或创建 `## Codex Workpad` 评论。
3. 将 issue 中的 `Scope`、`Acceptance Criteria`、`Validation` 同步到 workpad。
4. 记录当前基线：
   - 绝对路径
   - 当前分支
   - 短 SHA
   - `git status --short`

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

### Step 1.5: 指派开发角色

进入开发前，主 agent 需要做一次明确交接：

1. 确认该 issue 是否已经进入实现阶段。
2. 若是，将 issue 指派给 `张居正` 并切到 `In Progress`。
3. 创建一个开发子代理作为执行负责人。
4. 给开发子代理明确以下内容：
   - 对应的 Plane issue
   - 本次范围
   - 可写文件或模块边界
   - 最低验证要求
5. 在主 agent 侧保留对子代理输出的最终审阅权。

### Step 2: 实现

1. 保持 crate 边界清晰：
   - `imperial-desk-cli` 负责参数解析和输出
   - `imperial-desk-core` 负责共享类型和 trait
   - `imperial-desk-agent` 负责 provider-agnostic 协调逻辑
   - `imperial-desk-browser` 负责浏览器抽象与后端
   - `imperial-desk-state` 负责本地状态
   - `imperial-desk-provider` 负责厂商实现与注册
2. 不要把 DeepSeek 站点细节泄漏到 `imperial-desk-agent` 或 `imperial-desk-core`。
3. 架构或流程变更时，同时更新相关文档。
4. 子代理负责具体实现时，主 agent 负责：
   - 跟踪是否偏离 issue 范围
   - 处理跨模块协调
   - 统一本次提交内容
5. 子代理完成后，主 agent 必须复核其结果再进入验证或提交流程。

### Step 3: 验证

优先运行能直接证明本次改动的验证。

Rust 变更的默认验证顺序：

1. `$code-review`
2. `$code-simplify`
3. `cargo fmt --check`
4. `cargo test`
5. 如果改动集中在单个 crate，补充 `cargo test -p <crate>`
6. 如果涉及 CLI 面或 provider 注册，补充 `cargo run -p imperial-desk-cli -- providers`
7. 如果涉及 provider/browser 行为，补充手工 smoke 检查

如果未执行某项验证，必须在 workpad 和最终说明里写明原因。

### Step 3.5: 双审查交接

验证完成后：

1. 将 issue 路由给 `海瑞` 和 `于谦`。
2. 将 issue 切到 `Review`。
3. 创建两条独立的审查子代理链路执行 review。
4. 在 workpad 中分别记录两条审查结论：
   - `Review A: Pending | Pass | Changes Requested`
   - `Review B: Pending | Pass | Changes Requested`
5. 任一审查要求修改，都将 issue 退回 `Todo` 或 `In Progress`，重新交回 `张居正`。

### Step 4: 收尾

1. 将验证结果和审查结论回写到 workpad。
2. 只有在双审查都通过且当前验证证据完整时，才允许切到 `Done`。
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

1. 若已有分支和工作树，优先在与当前 issue 对应的范围内工作。
2. 每个 issue 至少对应一组清晰提交。
3. 不要把多个无关能力挤进同一次提交。
4. 默认不开启自动 commit、push、PR 或 auto-land。
5. 即使子代理完成了代码，最终提交仍由主 agent 统一整理和执行。

## 当前阶段提醒

实现状态必须以代码为准，而不是只以设计文档为准。

当前仓库已经具备一部分实现基础，因此后续 issue 需要区分：

- 已落地的能力补录为已完成 issue
- 仍是占位、骨架或未闭环的能力录入为 `Todo`

尤其要注意：

- `deepseek-web` 已有较多实现
- `imperial-desk-agent` 仍是极简版本，不等于完整工具协调器
- `deepseek-api` 目前仍是占位
- 测试覆盖几乎为空

## 后续开发默认模式

除纯文档调整、状态整理、轻量排查外，后续真实开发默认采用：

1. Plane issue 进入 `In Progress`
2. issue 指派给 `开发负责人`
3. 主 agent 创建开发子代理
4. 开发子代理接管实现
5. 主 agent 复核、验证、切到 `Review`
6. issue 进入 `Review` 并由 `海瑞`、`于谦` 两条审查链路接管
7. 双审查子代理完成 review
8. 主 agent 更新 Plane、提交 git、完成 close-out

这套流程是本仓库的默认开发模式，而不是临时约定。
