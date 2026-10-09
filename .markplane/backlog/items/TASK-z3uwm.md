---
id: TASK-z3uwm
title: Context 层与 MCP map resources
status: backlog
priority: medium
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-kjetr
blocks:
- TASK-d3tdm
related: []
assignee: null
tags:
- ready-for-agent
position: a8
created: 2026-10-09
updated: 2026-10-09
---

# Context 层与 MCP map resources

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

让 AI context 体现 maps 与 frontier。

- `summary.md`：Now 之后新增 `## Active Maps`，每张一行 `MAP-x: 标题 — 已决策/总数, next: <首张 frontier 标题> (TASK-id)`，最多 3 张。
- 新增 `.context/maps.md`：每张 active map 一节，含 Destination 首段、frontier、已 claim 工单与负责人、被阻塞数、Not yet specified 条目数（约 300–500 token，独立于 summary 预算）；新增 `maps/INDEX.md`。
- Decision ticket 不进 Priority Queue、Backlog 计数、metrics、blocked-items；已 claim 的出现在 active-work 并标注 Map。
- MCP resources：`markplane://maps`（内容即 `.context/maps.md`）与 URI 模板 `markplane://map/{id}`。
- `markplane claude-md` 片段增加 `.markplane/maps/INDEX.md` 与 `.context/maps.md` 两行，标题仍为 `## Project Management`。

## Acceptance criteria

- [ ] sync 后 summary 含 Active Maps 且不超过 3 张；maps.md 与 maps/INDEX.md 生成
- [ ] Decision ticket 不出现在 Priority Queue / metrics / blocked items；已 claim 的出现在 active work 并带 Map
- [ ] MCP 可读取 `markplane://maps` 与 `markplane://map/{id}`
- [ ] `claude-md` 输出包含新增两行
- [ ] core 单元测试覆盖 context 生成与排除规则；MCP 资源测试

## Blocked by

- Decision ticket：add --map 与 ls --map [[TASK-kjetr]]
