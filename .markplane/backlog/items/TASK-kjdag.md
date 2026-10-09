---
id: TASK-kjdag
title: 独立 PLAN、Plan tags 与 spec 模板
status: in-progress
priority: high
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-ywvnb
blocks:
- TASK-62jjn
- TASK-u34gh
related: []
assignee: chuck
tags:
- ready-for-agent
position: aB
created: 2026-10-09
updated: 2026-10-09
---

# 独立 PLAN、Plan tags 与 spec 模板

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

让 PLAN 能作为独立 spec，并让 EPIC / PLAN 共享 spec 结构。

- `markplane plan [TASK_ID] [--title t] [--body-file f] [--tags …]`：不带 TASK_ID 时创建独立 PLAN；带 TASK_ID 行为同上游。
- Plan 新增 `tags` 字段；`epic` 支持 `--tags`。
- 新增 `spec` 模板（Problem Statement … Further Notes 七节），在 manifest 中同时注册到 epic 与 plan。
- `add --plan <PLAN>` 把 Task 挂到 PLAN（`Plan.implements` 自动维护）；`ls --plan <PLAN>` 列出其工单。
- MCP：`markplane_add` 支持 plan 类型（含独立 PLAN），`markplane_query` 支持 plan 过滤。

## Acceptance criteria

- [ ] `plan --title t --tags ready-for-agent` 创建独立 PLAN，frontmatter 含 tags
- [ ] `epic --tags x` 生效；使用 spec 模板创建的 EPIC / PLAN 正文为七节结构
- [ ] `add --plan P` 后 Task.plan 与 Plan.implements 双向一致；`ls --plan P` 只列该 PLAN 的工单
- [ ] 带 TASK_ID 的 `plan` 行为与上游一致（回归测试）
- [ ] CLI 集成测试与 MCP 对齐测试

## Blocked by

- 创建类命令支持 --body-file [[TASK-ywvnb]]
