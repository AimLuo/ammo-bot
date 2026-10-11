---
id: TASK-mzkqb
title: 跨 Project 轮转与 Slot 总览
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-ce5ps
plan: null
depends_on:
- TASK-cppqm
blocks:
- TASK-minmn
related: []
assignee: null
tags:
- ready-for-agent
position: b02
created: 2026-10-10
updated: 2026-10-11
---

# 跨 Project 轮转与 Slot 总览

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

多个 Project 同时开扫描时公平分享全局 Slot，人在任何看板上都能看到所有 agent 在做什么。

- 拾取跨 Project 轮流（每轮每个 Project 取一个），Project 内仍按既有顺序；轮转为 core 纯函数并有单元测试。调低并发上限只停止新拾取，不杀 Run。
- `GET /api/dispatcher` 补全：并发上限、已用 Slot、运行中与排队数、按 Project 分组的活动 Run。Slot 变化、显示状态变化、排队位次变化、并发上限变化时推 `dispatcher_changed`。
- 界面：看板顶部 Scan switch 旁 `Slots used/limit` 徽章（含排队，占满时警示色）；点开弹出 `running n · merge queue m · limit k`，按 Project 分组（当前 Project 在前），每行 ID、标题与 Run 状态，点击跳到对应 ticket；没有时 "No active runs"。Settings 本机卡片的 `In use n` 用真实数值。

## Acceptance criteria

- [ ] 两个 Project 各有多张候选、并发上限较小时，拾取在两者之间交替
- [ ] 运行中把并发上限调到低于在用数，已有 Run 继续，直到降到上限以下才拾取新票
- [ ] `GET /api/dispatcher` 的计数与分组正确，排队中的 Run 计入 Slot
- [ ] 徽章与弹出总览显示正确，可跳转到其他 Project 的 ticket；前端 typecheck、lint、生产构建通过
