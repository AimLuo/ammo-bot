---
id: TASK-gxxax
title: 'REVIEW: issues 后走一轮 fix'
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
position: b03
created: 2026-10-10
updated: 2026-10-10
---

# REVIEW: issues 后走一轮 fix

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

implement 以 `REVIEW: issues` 结束时，Dispatcher 在同一 worktree 新开一次 fix 会话修复，然后进入合并；只修一轮，不再 review。

- fix prompt（原型分支 `prototype/stage-prompts`）：内联最新一条 `agent-implement` 写的 `**Review**` 评论，未内联时让 agent 自己 `markplane show`；确信是误报可不修，在 `**Fix report**` 里说明，仍算 `FIX: done`；难修就 `FIX: blocked`。
- 流程：`REVIEW: issues` → fix → `FIX: done` → 进入 Merge queue；`FIX: blocked` 或其他失败按失败判定转人工。fix 用 fix Stage 的模型与超时快照。

## Acceptance criteria

- [ ] 假 agent implement 输出 `REVIEW: issues` 后恰好启动一次 fix 会话，工作目录是同一 worktree
- [ ] fix prompt 里包含最新的 `**Review**` 评论内容
- [ ] `FIX: done` 后合并并标为 done；`FIX: blocked` 转人工且原因指向 fix
- [ ] 不会在 fix 之后再次 review 或循环
