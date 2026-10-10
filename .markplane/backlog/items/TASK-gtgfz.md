---
id: TASK-gtgfz
title: 看板 RunBadge、失败 toast 与改状态确认
status: planned
priority: medium
type: feature
effort: medium
epic: EPIC-ce5ps
plan: null
depends_on:
- TASK-in6ej
- TASK-hreg4
blocks: []
related: []
assignee: null
tags:
- ready-for-agent
position: b07
created: 2026-10-10
updated: 2026-10-10
---

# 看板 RunBadge、失败 toast 与改状态确认

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

看板上能直接看到每张 ticket 的 Run 在做什么；Run 失败时人会被提醒；人接手有活动 Run 的 ticket 前会被告知后果。

- `GET /api/runs`（Project 前缀下）：每张有记录 ticket 最新 Run 的摘要（task_id、序号、显示状态、Stage、Stage 开始时间、Run 开始时间、排队位次、失败原因、结束时间）。显示状态由服务端统一计算（core 纯函数，有单元测试）；客户端只额外派生 stopping。
- RunBadge：卡片底部 assignee 行右侧，Needs You 行与 Dashboard Active Work 复用；文案与显示条件见父 Spec「Run 界面」（`implement · 12m`、`merge queue #2 · 5m`、`Stopping · implement`、`Run failed · merge`、`Run stopped` 等，全页 30 秒计时器，succeeded 不显示，failed / stopped 只在 ticket 仍在 active 时显示）；悬停提示给出 Run 序号、总用时、Stage 开始时间、模型与超时、调用次数；遵守 `prefers-reduced-motion`。
- WebSocket：`run_changed` 使 `runs` 查询失效，`dispatcher_changed` 使 `dispatcher` 查询失效；重连成功（非首次）时让全部查询失效。当前 Project 收到 failed 时 toast `TASK-xxx run failed at <stage>`，附 "Open"。
- 确认框：有活动 Run 的 ticket 被拖出 active 列、或在详情面板改状态或 assignee 时先确认，说明当前 Stage 结束后停止、不杀进程、保留 worktree；改其他字段不确认。

## Acceptance criteria

- [ ] `GET /api/runs` 摘要与显示状态正确（进程内 API 测试 + 显示状态单元测试）
- [ ] 卡片、Needs You 行、Dashboard 上的 RunBadge 随 Run 推进实时变化，用时按分钟更新
- [ ] Run 失败时当前 Project 弹 toast，点 "Open" 打开该 ticket
- [ ] 拖出 active 列或改状态 / assignee 时弹确认框，取消则不改；改其他字段不弹
- [ ] 断开并重连 WebSocket 后看板数据刷新
- [ ] 前端 typecheck、lint、生产构建通过
