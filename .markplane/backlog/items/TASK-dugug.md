---
id: TASK-dugug
title: .context 摘要与 MCP resources 如何包含 map
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-vs3wh
depends_on: []
blocks: []
related: []
assignee: null
tags: []
position: aP
created: 2026-10-09
updated: 2026-10-09
---

# .context 摘要与 MCP resources 如何包含 map

## Question

`.context/`（summary.md、active-work.md、blocked-items.md、metrics.md、INDEX.md）与 MCP resources 是否、如何体现 MAP、决策工单与 frontier？例如 summary 是否列出 active maps 及其 frontier，blocked-items 是否混入决策工单，token 预算如何分配？

## Answer

2026-10-09 与用户 grilling 确认（全部采纳推荐）：

1. **`summary.md`**：在 Now 之后新增 `## Active Maps`，每张 active map 一行：`MAP-x: 标题 — 已决策数/总数, next: <首张 frontier 工单标题> (TASK-id)`，最多 3 张。
2. **决策工单在其他摘要中**：Priority Queue、Backlog 计数、`metrics.md`、`blocked-items.md` 均排除（与看板默认隐藏一致）；`active-work.md` 包含已 claim 的决策工单并标注所属 map。
3. **新增 `.context/maps.md`**：每张 active map 一节——Destination 首段、frontier 列表（标题 + ID）、已 claim 工单与负责人、被阻塞数、Not yet specified 条目数；约 300–500 token，不占 summary 的 1000 token 预算。并像其他目录一样生成 `maps/INDEX.md`。
4. **MCP resources**：新增 `markplane://maps`（内容即 `maps.md`）与 URI 模板 `markplane://map/{id}`（返回 MAP 全文）；`markplane_context` 不加 map focus。
