# Imperial Desk Agent Guide

本仓库是 `imperial-desk` 的主实现仓库。
项目目标是构建一个 Rust 版 web LLM adapter，当前重点是 `deepseek-web`，并保持 CLI、agent 协调器和 provider 边界清晰。

## 仓库定位

- 任务系统：Plane
- 源码主仓库：当前仓库
- 主设计文档：`docs/design.md`
- 协作规则：`WORKFLOW.md`

不要把别的项目的上游依赖、fork 策略、固定 worktree 流程照搬到这里。

## 当前实现快照

开始工作前，先按代码确认现状。
截至当前仓库状态，可直接观察到：

- 已有 Rust workspace 和六个核心 crate
- CLI 已实现 `providers`、`login`、`config`、`ask`、`agent`、`inspect`、`delete*`
- `imperial-desk-browser` 已有 Chromium/CDP backend
- `imperial-desk-state` 已有 profile、recent session、provider config 持久化
- `deepseek-web` 已有 `ask`、短信登录、`inspect` 和删除接口实现
- `imperial-desk-agent` 目前仍是最小骨架，尚未实现完整 tool loop
- `deepseek-api` 仍是占位实现
- 测试已能跑通，但当前几乎没有测试用例

因此不要把“设计里计划有的东西”当成“代码里已经完成的东西”。

## Tracker 规则

- Plane 是唯一任务事实来源。
- 所有实际实现、重构、流程变更，都应该能对应到一个 Plane issue。
- 所有 Plane 项目内容统一使用中文：
  - issue 标题
  - issue 描述
  - `## Codex Workpad`
  - review 总结
  - 完成说明
- 每个活跃 issue 保持一个持续更新的 `## Codex Workpad` 评论。
- 新发现的超范围内容要新建 follow-up issue，不要悄悄并入当前 issue。

## 当前角色标准

本项目以 `WORKFLOW.md` 为角色标准，当前 Plane 侧约定为四层角色：

- 统筹角色
  - `Coordinator`
  - 当前对应 `徐阶`
  - 负责 issue 路由、workpad、状态和交付就绪判断
- 业务实现角色
  - `Development Owner`
  - 当前对应 `张居正`
  - `In Progress` 阶段的 Plane assignee
- 业务审查角色 A
  - `Review Owner A`
  - 当前对应 `海瑞`
  - `Review` 阶段的第一条审查 assignee
- 业务审查角色 B
  - `Review Owner B`
  - 当前对应 `于谦`
  - `Review` 阶段的第二条审查 assignee

执行层仍由子代理接管：

- 开发子代理代表 `Development Owner` 执行实现和自验证
- 审查子代理 A 代表 `Review Owner A` 执行独立审查
- 审查子代理 B 代表 `Review Owner B` 执行独立审查

约束：

- 统筹角色不是默认开发 assignee，也不是默认审查 assignee
- 不把虚拟成员直接写成 Plane 用户
- 不让子代理直接替代 Plane 的项目所有权
- 每个实现中的 issue 默认只有一个开发执行负责人
- 多子代理并行时必须有明确边界和文件所有权

## 架构边界

修改代码时遵守以下边界：

- `imperial-desk-cli`
  - 只负责参数解析、命令分发、终端交互、输出格式
- `imperial-desk-core`
  - 只放共享类型、错误、trait、capability 契约
- `imperial-desk-agent`
  - 只放 provider-agnostic agent 协调逻辑
- `imperial-desk-browser`
  - 只放浏览器后端抽象和自动化能力
- `imperial-desk-state`
  - 只放本地状态路径、配置和持久化
- `imperial-desk-provider`
  - 只放 provider 注册和厂商实现

DeepSeek 的 DOM 选择器、页面逻辑、接口细节必须留在 `imperial-desk-provider` 中。
不要把 provider 特定逻辑塞进 `imperial-desk-agent` 或 `imperial-desk-core`。

## 实现规则

- 优先做能力闭环，而不是表面命令补齐。
- 跨 crate 改动时，先确认契约变化，再改调用侧。
- 架构、流程、命令面变化时，同步更新文档。
- 对未完成能力保持诚实，不要把占位实现包装成完整支持。

尤其注意以下高风险误判：

- `agent` 命令存在，不代表工具执行层已完整实现
- `deepseek-api` 模块存在，不代表 provider 已可用
- `login` 存在，不代表交互式 wizard 和 WeChat QR 已完成

## 文档同步规则

如果变更涉及以下内容，必须同步文档：

- CLI surface
- provider capability
- login 流程
- agent protocol
- crate ownership
- 本地状态模型
- Plane 角色路由与工作流

优先同步的文档：

- `docs/design.md`
- `WORKFLOW.md`
- `AGENTS.md`

如果本次工作包含可复用的系统/环境/认证/追踪器维护操作，也必须同步更新对应 skill 或 runbook。

- 尤其是本地 Plane 用户、成员、角色账号维护
- 优先回写 `C:\Users\windo\.agents\skills\system-ops-runbook`
- 目标是减少下次重复探索、降低出错率、降低 token 消耗

## 验证要求

Rust 代码改动的最低验证要求：

- `$code-review`
- `$code-simplify`
- `cargo fmt --check`
- `cargo test`

如果只改单个 crate，也可以先跑针对性验证，但跨 crate 契约变更最终仍应回到工作区级验证。

如果因为外部依赖、真实站点登录态或环境限制无法完成验证，必须明确记录：

- 哪一步没跑
- 为什么没跑
- 剩余风险是什么

## Git 规则

- 默认主分支为 `main`
- 每个 Plane issue 对应清晰提交
- 提交前确认变更范围与 issue 一致
- 未经明确要求，不要重写历史
- 不要因为“顺手”修改与当前 issue 无关的文件

## Agent 协作方式

- 主 agent 默认负责统筹
- 可以把有明确边界的探索、验证或局部实现分配给子 agent
- 并行委派时避免重叠写文件
- 子 agent 的发现需要回写到主 issue 的 workpad 或最终总结中

### 开发接管规则

对于进入实现阶段的 issue，默认流程不是“主 agent 亲自写全部代码”，而是：

1. 主 agent 先同步 Plane issue、范围和验证要求。
2. 将 issue 指派给 `张居正`，并切到 `In Progress`。
3. 主 agent 创建一个开发子代理，作为该 issue 的执行负责人。
4. 开发子代理负责：
   - 读取相关代码和文档
   - 在授权范围内完成实现
   - 运行最低限度验证
   - 回报改动文件、结果和风险
5. 主 agent 负责：
   - 控制范围
   - 复核子代理产出
   - 必要时补充或修正实现
   - 更新 Plane 与 git

### 审查接管规则

进入审查阶段后：

1. 将 issue 切到 `Review`，并路由给 `海瑞` 与 `于谦` 两条审查链路。
2. 主 agent 创建两个互相独立的审查子代理执行 review。
3. 任一审查子代理都可以提出阻塞性发现。
4. 主 agent 负责处理回流、更新 workpad，并在双审查都非阻塞时才允许交付。

### 何时必须创建执行负责人子代理

以下场景默认必须创建子代理：

- 新功能开发
- 中等以上规模重构
- 需要跨多个 crate 的实现
- 明确属于 Plane 中某个后续开发 issue 的工作

以下场景可以不创建：

- 纯文档同步
- 轻量配置调整
- 简单排障或信息核对
- 只需极小改动的一次性修正

### 子代理边界规则

- 子代理必须拿到明确的 issue 范围
- 子代理必须知道自己负责哪些文件或模块
- 子代理不应随意扩展到未分配的 crate 或功能
- 子代理不直接决定关闭 issue，关闭权在主 agent
- 子代理完成后，主 agent 必须审阅再合并结果

## 本项目的优先级判断

如果需要在多个方向之间排序，优先级通常是：

1. 让已有实现可验证、可维护
2. 补齐 Phase 1 关键闭环
3. 再扩展新 provider 或新 transport

按当前仓库现实，优先关注：

1. 文档和流程与现状对齐
2. Plane issue 体系补录
3. agent/tool loop 补全
4. login wizard 与二维码登录
5. 测试覆盖和诊断
6. `deepseek-api` 真正落地

<!-- plane-role-orchestration:begin -->
## Plane Role Orchestration Rules

This managed section is maintained by the user-level skill
`plane-role-orchestration`.

Keep repo-specific architecture, module ownership, and product details outside
this section, but keep execution policy here aligned with `WORKFLOW.md`.

### Source Of Truth

- Plane is the primary tracker.
- `WORKFLOW.md` defines state flow, role routing, and runner defaults.
- This file defines repo-local agent behavior, execution guardrails, and delivery discipline.
- If `WORKFLOW.md` and this section diverge, update both in the same change.

### Role Separation

- `Coordinator` routes work, maintains the workpad, and checks delivery readiness.
- `Development Owner` owns implementation and self-validation.
- `Review Owner A` and `Review Owner B` are independent review lanes.
- Do not let the coordinator silently perform development or review work unless the repo explicitly allows it.

### Plane Protocol

- Do not start implementation without a corresponding Plane issue.
- Keep one persistent `## Codex Workpad` comment per active issue.
- Mirror `Scope`, `Acceptance Criteria`, and `Validation` into that workpad before editing.
- Record handoffs, review outcomes, blockers, and validation evidence in the same workpad.

### Development Owner Rules

- Keep changes scoped to the active issue.
- Before self-validation, run `$code-review` against the implementation diff.
- Apply or explicitly disposition review findings before continuing.
- Run `$code-simplify` on the same scope after review findings are handled.
- Re-run the narrow validation needed to prove the simplification pass preserved behavior.
- Only hand the issue to review after current validation evidence exists.

### Review Gate

- Treat Review A and Review B as independent checks.
- If either review lane requests changes, route the issue back to implementation.
- Do not mark work ready for delivery until all required review lanes are non-blocking.
- Do not close or land work when tracker state, workpad state, validation, and git state disagree.

### Worktree And Git Hygiene

- Start from a clean or issue-scoped worktree.
- Do not mix unrelated issue work in the same branch, commit, or worktree.
- Before committing, verify the staged diff matches the active issue only.
- Prefer repo-local git skills when available: `$pull`, `$commit`, `$push`, `$land`.
- Do not rewrite or discard unrelated local changes without explicit approval.

### Docs And Validation

- Update `WORKFLOW.md`, `AGENTS.md`, and relevant design docs when behavior, interfaces, ownership, or delivery policy changes.
- Run the narrowest validation that proves the touched behavior is correct.
- If validation is skipped, record the reason in the workpad and final summary.
- Create follow-up Plane issues for meaningful scope growth instead of silently widening the current issue.
<!-- plane-role-orchestration:end -->
