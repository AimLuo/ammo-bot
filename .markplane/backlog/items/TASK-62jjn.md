---
id: TASK-62jjn
title: Frontier 查询与 cancelled blocker 视为解除
status: backlog
priority: high
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-kjdag
blocks:
- TASK-kjetr
- TASK-k4tfw
related: []
assignee: null
tags:
- ready-for-agent
position: aC
created: 2026-10-09
updated: 2026-10-09
---

# Frontier 查询与 cancelled blocker 视为解除

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

实现 frontier 查询，并修正阻塞判定。

- 阻塞判定（全局，如 `find_blocked_items`）把 completed 与 cancelled 的 blocker 都视为已解除。
- Frontier：父范围内 open、所有 `depends_on` 已关闭、assignee 为空的 Task，按 `position` 排序。
- `ls --epic X --frontier` / `ls --plan X --frontier` 返回 frontier。
- MCP `markplane_query` 支持 frontier 过滤，结果补 `assignee`、`depends_on`、`tags`。

状态语义按 StatusCategory 判断，不写死状态名。

## Acceptance criteria

- [ ] blocker 被 cancelled 后，被阻塞工单进入 frontier，且不再出现在 blocked items
- [ ] 已 claim（assignee 非空）或有未关闭依赖的工单不在 frontier
- [ ] frontier 按 `position` 排序
- [ ] MCP query 的 frontier 结果与 CLI 一致，且返回 assignee / depends_on / tags
- [ ] core 单元测试覆盖 frontier 计算与 cancelled 处理；CLI 集成测试与 MCP 对齐测试

## Blocked by

- 独立 PLAN、Plan tags 与 spec 模板 [[TASK-kjdag]]
