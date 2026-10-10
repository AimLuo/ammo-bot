---
id: TASK-rhzty
title: Dispatcher 自动关闭 Scan switch
status: planned
priority: medium
type: feature
effort: small
epic: EPIC-ce5ps
plan: null
depends_on:
- TASK-in6ej
blocks: []
related: []
assignee: null
tags:
- ready-for-agent
position: b05
created: 2026-10-10
updated: 2026-10-10
---

# Dispatcher 自动关闭 Scan switch

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

Dispatcher 在明显出问题时自己关掉 Scan switch，避免烧光队列或在失去锁后继续工作。

- 启动失败：同一 Project 连续 3 次 Stage 调用在拿到 `system/init` 前就结束，自动关闭该 Project 的 Scan switch（阈值写死）；有一次拿到 init 即清零。
- 锁丢失：每次扫描核对锁文件是否还是同一个文件，不一致就当作锁已丢失，关闭 Scan switch、停止拾取。
- 两种情况都在内存里记原因、写 `serve.log`、推 `dispatcher_changed`；`GET /api/dispatcher` 返回自动关闭原因，界面在开关旁显示。

## Acceptance criteria

- [ ] 假 agent 连续 3 次不输出 init 就退出后，Scan switch 自动关闭并给出原因；中间夹一次成功 init 则计数清零
- [ ] 删除或替换锁文件后下一次扫描关闭 Scan switch 并告警
- [ ] 开关旁显示自动关闭原因；手动重新打开后原因清除
