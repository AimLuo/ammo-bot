---
id: TASK-d7i23
title: 详情面板完整 Run 段
status: planned
priority: medium
type: feature
effort: medium
epic: EPIC-ce5ps
plan: null
depends_on:
- TASK-in6ej
- TASK-gbq58
blocks:
- TASK-minmn
- TASK-kkmrn
related: []
assignee: null
tags:
- ready-for-agent
position: b06
created: 2026-10-10
updated: 2026-10-10
---

# 详情面板完整 Run 段

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

补全详情面板的 Run 段，人不开文件就能诊断一次 Run。

- 头部：RunBadge、`Run #n`、开始时间、总用时、操作位（已有 View sessions；Resume、Continue…、Clean up… 由后续票放入）。
- Stage 进度 `implement → fix → merge`（fix 只在 `REVIEW: issues` 时出现），每步结果图标、模型、用时、`in / out` token。
- 失败原因全文；遗留物状态行（如 `Worktree kept · 3 uncommitted files`）；Details 折叠区：worktree 路径（可复制）、分支、基点 SHA、目标分支、合并 SHA、是否推送、每次调用的用量与 Run 合计，已删路径标 `(removed)`。
- "Earlier runs (n)" 折叠区列出更早的 Run。

## Acceptance criteria

- [ ] 成功、失败、经过 fix 的 Run 分别正确显示 Stage 进度与每步结果、模型、用时、token
- [ ] 失败 Run 显示完整原因和遗留物状态
- [ ] Details 内容完整，worktree 路径可复制，已清理路径标 `(removed)`
- [ ] 有多次 Run 时更早的 Run 在折叠区里
- [ ] 前端 typecheck、lint、生产构建通过
