---
id: TASK-hkj4u
title: Dispatcher 设置与 Settings 的 Dispatcher 标签页
status: planned
priority: medium
type: feature
effort: medium
epic: EPIC-ce5ps
plan: null
depends_on: []
blocks:
- TASK-cppqm
related: []
assignee: null
tags:
- ready-for-agent
position: av
created: 2026-10-10
updated: 2026-10-10
---

# Dispatcher 设置与 Settings 的 Dispatcher 标签页

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

Dispatcher 的设置可以在 Settings 里查看和修改，随仓库共享的与按机器的分开存放，改动热生效；文档写明无人值守的安全边界。

- 共享设置：主工作区 `config.yaml` 的 `dispatcher.push_after_merge`（默认开）、`dispatcher.stages.<implement|fix|merge>.model`（默认 `cursor-grok-4.5-high`，非空、无空白、≤200 字符）、`timeout_minutes`（默认 60，1–1440）。本机设置：用户配置目录 `serve.json` 的 `dispatcher.concurrency`（默认 20，1–100）。全部可省略，未显式设置的项不写回。本仓的 `config.yaml` 写 `push_after_merge: false`。
- `/api/config` 的 GET / PATCH 增加 `dispatcher` 段；并发上限与模型列表用全局端点（与 `/api/projects` 同级）。越界返回 400 `validation_error`；手改出非法值时沿用上次有效配置并在 `serve.log` 记警告；`serve.json` 读不出用默认值。改动（含 `git pull` 带来的）无需重启即生效，提供"取当前有效设置快照"的入口供 Stage 开跑时使用。
- 模型列表来自 `agent models` 的文本解析（`NO_COLOR=1`、去 ANSI、逐行匹配 `^(\S+) - (.+)$`、去 ` (default)` / ` (current)`），serve 内缓存约 10 分钟，失败时为空。解析为 core 纯函数并有单元测试。
- Settings 新增 Dispatcher 标签页：卡片 "This Project (shared via config.yaml)"（推送开关、三个 Stage 的模型与超时，模型字段为下拉加自由输入，不在列表里只警告）与 "This machine (all Projects)"（并发上限，`In use n` 在 Slot 计数存在前可显示 0）；一行安全说明（agent 以完全权限运行）。
- 文档：Dispatcher 安全说明（见父 Spec「文档」节）与"一个仓库只在一个地方开扫描"。

## Acceptance criteria

- [ ] 未配置时 GET 返回各项默认值；PATCH 后 `config.yaml` / `serve.json` 只写出显式设置的项
- [ ] 越界值返回 400 `validation_error`；手改非法值后沿用上次有效配置并记警告
- [ ] 直接改文件后，不重启 serve 即读到新值
- [ ] `agent models` 文本解析的单元测试覆盖默认/当前后缀与 ANSI 码
- [ ] Settings 的 Dispatcher 标签页两张卡片可编辑、保存、显示校验错误与安全说明
- [ ] 文档包含安全说明
- [ ] 进程内 API 测试覆盖读写与校验；前端 typecheck、lint、生产构建通过
