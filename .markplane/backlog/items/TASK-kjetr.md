---
id: TASK-kjetr
title: Decision ticket：add --map 与 ls --map
status: in-progress
priority: high
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-62jjn
- TASK-c7x96
blocks:
- TASK-gmgsq
- TASK-d5gnj
- TASK-z3uwm
- TASK-2uakq
related: []
assignee: chuck
tags:
- ready-for-agent
position: aE
created: 2026-10-09
updated: 2026-10-09
---

# Decision ticket：add --map 与 ls --map

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

让 Task 能作为挂在 Map 下的 **Decision ticket**。

- `add --map <MAP> --type research|prototype|grilling|task`：有 `map` 时类型校验改为这四个内置值，跳过 `task_types` 配置（config 不新增这些值）。
- `map` 与 `epic` / `plan` 互斥，由 core 校验（add、link、update 各路径）。
- `link <TASK> <MAP> -r map` 设置或重新挂到另一 Map；关系只记在 Task 侧。
- `ls --map <MAP> [--frontier]` 列出工单 / frontier（复用 frontier 逻辑）。
- claim 即 `start --user <name>`；范围外即 cancel。Decision ticket 使用统一默认 priority。
- MCP：`markplane_add` 支持 map 参数，`markplane_query` 支持 map 过滤。

## Acceptance criteria

- [ ] `add --map M --type grilling` 成功；`--type feature` 被拒绝；配置之外的四个类型无需改 config
- [ ] 同时设置 map 与 epic/plan 被拒绝（add 与 link 路径）
- [ ] `link T M2 -r map` 后 Task.map 变为 M2
- [ ] `ls --map M --frontier` 排除已 claim、被阻塞、已关闭工单，按 position 排序；cancelled blocker 视为解除
- [ ] CLI 集成测试与 MCP 对齐测试

## Blocked by

- Frontier 查询与 cancelled blocker 视为解除 [[TASK-62jjn]]
- Map 条目类型 [[TASK-c7x96]]
