---
id: TASK-2uakq
title: Web UI：/maps 列表与路线图详情页
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-kjetr
blocks:
- TASK-mbc9k
- TASK-9hqi2
related: []
assignee: chuck
tags:
- ready-for-agent
position: a9
created: 2026-10-09
updated: 2026-10-09
---

# Web UI：/maps 列表与路线图详情页

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

在 Web UI 中可视化 Map（原型分支 `prototype/map-page`，选定 Variant C 路线图）。

- `/maps` 列表页：卡片显示 ID、状态、标题、进度（已决策/总数）、frontier 数。
- Map 详情页：Decision ticket 按依赖深度从左到右排列，从 Start 到 Destination；节点显示类型徽章与标题，边框编码 decided / claimed / frontier / blocked；cancelled 工单不进图，列在 Out of scope。
- 点击节点在侧栏显示 Question、Answer、blockers；侧栏下方显示 Not yet specified 与 Out of scope。
- serve 提供所需 API。

原型中的状态判定（来自原型）：

```ts
type TicketState = "decided" | "claimed" | "frontier" | "blocked" | "out-of-scope";
// cancelled → out-of-scope; done → decided; assignee → claimed;
// any dependsOn not closed (done|cancelled) → blocked; else frontier
// layout: depth = 0 if no deps, else 1 + max(depth of deps); column = depth
```

## Acceptance criteria

- [x] 导航可进入 `/maps`，卡片字段完整
- [x] 详情页布局与节点状态符合上述判定；点击节点显示侧栏
- [x] `tsc`、lint、`npm run build` 通过；serve 相关 Rust 测试通过
- [x] 用测试数据人工核对页面（真实数据核对在迁移工单完成）

## Blocked by

- Decision ticket：add --map 与 ls --map [[TASK-kjetr]]
