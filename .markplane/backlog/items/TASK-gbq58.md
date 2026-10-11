---
id: TASK-gbq58
title: Run record 格式、只读接口、最小 Run 段与会话弹窗列表
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-24iwg
plan: null
depends_on: []
blocks:
- TASK-grji3
- TASK-cppqm
- TASK-d7i23
related: []
assignee: null
tags:
- ready-for-agent
position: at
created: 2026-10-10
updated: 2026-10-11
---

# Run record 格式、只读接口、最小 Run 段与会话弹窗列表

## Parent

任务卡片评论、Needs You 与会话记录 [[EPIC-24iwg]]

## What to build

定义 Run record 格式并提供只读接口；详情面板出现最小 Run 段，View sessions 打开会话记录弹窗并在左侧列出 Run → Stage → 会话 → 调用。本票不写 Run record（那是 Dispatcher 的事），用 fixture 验证。

- Run record：每张 Implementation ticket 一个 `.markplane/.runs/<TASK-id>.json`，目录自带内容为 `*` 的 `.gitignore`（做法同条目锁目录）；字段见父 Spec「Run record 格式」（`runs` 数组、Run 字段与清理字段、`attempts` 数组及其 `usage` 等）。core 提供类型与读取，以及供 Dispatcher 使用的原子写入（临时文件 + rename）。文件监听不因它推事件。
- `GET /api/runs/{task_id}`（Project 前缀下）：返回完整记录，没有时返回空数组。
- Run 段：本机有记录时出现在元数据和描述之间，头部右侧操作位只放 "View sessions"；assignee 是 `dispatcher` 但本机没有记录时只显示灰字 "No local Run record — it ran on another machine" 和 View sessions。
- 会话记录弹窗（原型分支 `prototype/session-modal` 的 Variant A，重写不搬代码）：大尺寸居中；左侧最新 Run 在前 → Stage 按 implement → fix → merge → 按会话 ID 分组 → 各次调用，标出触发方式（initial / resume / continue）和 `resume_mismatch`；右侧本票只放选中会话的占位，正文在后续票。空状态 "本机没有 Run 记录"。已归档 Task 同样可用。

## Acceptance criteria

- [ ] `.runs/` 目录创建时自带 `.gitignore`，`git status` 看不到 Run record
- [ ] `GET /api/runs/{task_id}` 对 fixture 记录返回完整内容、无记录时返回空数组（进程内 API 测试）
- [ ] 有记录的 Task 显示 Run 段；assignee 为 `dispatcher` 而无记录时显示灰字提示
- [ ] 弹窗左侧按规定顺序分组列出调用，标出触发方式与 `resume_mismatch`；无记录时显示空状态
- [ ] 前端 typecheck、lint、生产构建通过
