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

## 负责人模型

本项目把“负责人”拆成两个角色：

- Plane 负责人
  - 挂在 Plane 项目或 issue 上的人类账号
  - 负责项目所有权、排期、状态和验收记录
- 执行负责人
  - 由主 agent 创建的子代理
  - 负责接管某个开发 issue 的具体实现

约束：

- 不把虚拟成员直接写成 Plane 用户
- 不让子代理直接替代 Plane 的项目所有权
- 每个实现中的 issue 默认只有一个执行负责人
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

优先同步的文档：

- `docs/design.md`
- `WORKFLOW.md`
- `AGENTS.md`

## 验证要求

Rust 代码改动的最低验证要求：

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
2. 主 agent 创建一个子代理，作为该 issue 的执行负责人。
3. 子代理负责：
   - 读取相关代码和文档
   - 在授权范围内完成实现
   - 运行最低限度验证
   - 回报改动文件、结果和风险
4. 主 agent 负责：
   - 控制范围
   - 复核子代理产出
   - 必要时补充或修正实现
   - 更新 Plane 与 git

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
