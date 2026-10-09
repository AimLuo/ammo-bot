---
id: TASK-gmgsq
title: resolve 命令
status: backlog
priority: high
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-kjetr
blocks:
- TASK-k4tfw
- TASK-9hqi2
related: []
assignee: null
tags:
- ready-for-agent
position: aF
created: 2026-10-09
updated: 2026-10-09
---

# resolve 命令

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

一步记录决策结论。`markplane resolve <TASK> --answer-file f --gist "<一句话>"` 原子地：

1. 在工单正文写入 `## Answer`（内容来自文件）；
2. 将工单置为 completed 状态；
3. 在所属 Map 的 Decisions so far 节追加 `- 标题 [[ID]]: gist`。

若 Map 已无 open 工单，打印提示（提醒询问用户是否 `status MAP done`），不自动改 Map 状态。新增 MCP `markplane_resolve`。

## Acceptance criteria

- [ ] resolve 后工单含 `## Answer` 且状态为 done，Map 的 Decisions so far 多出一行正确格式的条目
- [ ] 最后一张 open 工单 resolve 时输出提示，Map 状态不变
- [ ] 对非 Decision ticket 执行 resolve 给出明确错误
- [ ] MCP `markplane_resolve` 与 CLI 结果一致
- [ ] CLI 集成测试与 MCP 对齐测试

## Blocked by

- Decision ticket：add --map 与 ls --map [[TASK-kjetr]]
