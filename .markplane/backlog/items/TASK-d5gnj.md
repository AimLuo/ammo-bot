---
id: TASK-d5gnj
title: Map 归档连带工单与 link -r spec
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-kjetr
blocks:
- TASK-u34gh
- TASK-k4tfw
- TASK-9hqi2
related: []
assignee: chuck
tags:
- ready-for-agent
position: a7
created: 2026-10-09
updated: 2026-10-09
---

# Map 归档连带工单与 link -r spec

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

- `markplane archive <MAP>` 连带归档其全部 Decision ticket；Map 状态变化不触发任何自动动作。
- `link <MAP> <SPEC> -r spec` 维护 Map 的 `specs` 字段，只接受 EPIC / PLAN；EPIC / PLAN 不加反向字段。

## Acceptance criteria

- [ ] 归档 Map 后其 Decision ticket 均在 archive 中，`ls --map M` 为空
- [ ] `link M EPIC-x -r spec` 写入 `specs`；指向 TASK 被拒绝；`--remove` 可移除
- [ ] CLI 集成测试

## Blocked by

- Decision ticket：add --map 与 ls --map [[TASK-kjetr]]
