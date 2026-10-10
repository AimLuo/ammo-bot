---
id: TASK-v4j2t
title: CLI 支持 MARKPLANE_ROOT
status: planned
priority: medium
type: feature
effort: small
epic: EPIC-ce5ps
plan: null
depends_on: []
blocks:
- TASK-cppqm
related: []
assignee: null
tags:
- ready-for-agent
position: au
created: 2026-10-10
updated: 2026-10-10
---

# CLI 支持 MARKPLANE_ROOT

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

CLI 支持环境变量 `MARKPLANE_ROOT`：设置后从该目录找 `.markplane/`，不再从当前目录向上找。Dispatcher 靠它让 worktree 里的 agent 把评论写进主工作区的 ticket 文件。

## Acceptance criteria

- [ ] 在另一个目录（如 worktree）里设置 `MARKPLANE_ROOT=<主工作区>` 运行 `markplane comment` / `markplane show`，读写的是主工作区的 `.markplane/`
- [ ] 未设置时行为不变（从当前目录向上找）
- [ ] `MARKPLANE_ROOT` 指向没有 `.markplane/` 的目录时给出清楚的错误
- [ ] CLI 集成测试覆盖上述情况
