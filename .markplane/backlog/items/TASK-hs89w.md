---
id: TASK-hs89w
title: 评论默认作者统一并支持 MARKPLANE_AUTHOR，追加时标题降级
status: planned
priority: medium
type: feature
effort: small
epic: EPIC-24iwg
plan: null
depends_on: []
blocks:
- TASK-h2vcn
- TASK-cppqm
related: []
assignee: null
tags:
- ready-for-agent
position: as
created: 2026-10-10
updated: 2026-10-10
---

# 评论默认作者统一并支持 MARKPLANE_AUTHOR，追加时标题降级

## Parent

任务卡片评论、Needs You 与会话记录 [[EPIC-24iwg]]

## What to build

评论的默认作者在 CLI、MCP、serve 三处统一，Dispatcher 能通过环境变量给 agent 署名；任何入口追加的评论都不会破坏 `## Comments` 段。

- 默认作者取值规则收进 core 共用：`MARKPLANE_AUTHOR` → `$USER` → `$USERNAME` → `me`。CLI 的 `markplane comment`、MCP 的评论工具改用它（`start` 等用作 assignee 默认值的地方保持原有行为，除非规则本就相同）。
- 追加评论时，新评论文本里代码围栏外的 H1–H3 标题行一律降为 H4；CLI、MCP 两个入口规则一致（Web 入口在后续票复用同一函数）。

## Acceptance criteria

- [ ] 设置 `MARKPLANE_AUTHOR=agent-implement` 时 `markplane comment` 写出的评论作者是 `agent-implement`，未设置时回退到 `$USER` / `$USERNAME` / `me`
- [ ] MCP 评论工具在未传 author 时遵守同一规则
- [ ] 评论文本里的 `#` / `##` / `###` 标题被降为 `####`，代码围栏内的标题行不变
- [ ] core 单元测试覆盖作者取值顺序与标题降级；CLI 集成测试覆盖 `MARKPLANE_AUTHOR`
