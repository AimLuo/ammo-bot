---
id: TASK-grji3
title: 会话弹窗显示完整会话记录
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-24iwg
plan: null
depends_on:
- TASK-gbq58
blocks: []
related: []
assignee: null
tags:
- ready-for-agent
position: ay
created: 2026-10-10
updated: 2026-10-11
---

# 会话弹窗显示完整会话记录

## Parent

任务卡片评论、Needs You 与会话记录 [[EPIC-24iwg]]

## What to build

会话记录弹窗右侧显示所选会话的完整记录，人能看清 agent 说了什么、做了什么、花了多少 token。

- 会话记录接口（Project 前缀下，按 ticket ID 和会话 ID 定位）：读 `${CURSOR_DATA_DIR:-~/.cursor}/projects/<slug>/agent-transcripts/<id>/<id>.jsonl`；slug 由该次调用的工作目录正向计算（非 `[A-Za-z0-9]` 换成 `-`、合并连续 `-`、去首尾 `-`），记录里有已解析 transcript 路径时优先用它。每次请求整文件重读；行格式与末行 `turn_ended` 见父 Spec；文件不存在、读不了或末行截断时返回明确的"不可用"结果而非错误。
- 右侧：完整会话，用分隔线标出每次调用，点左侧调用滚到对应位置；Dispatcher prompt、agent 文本、工具调用三类分别开关；工具调用每轮折成一行可展开看输入；只显示最后 60 条，顶部 "show earlier" / "show all"。
- 用量：每次调用显示输入 / 输出 token，会话和 Run 显示合计，有调用用量未知时合计前加 `≥`；缓存明细放悬停提示。
- 头部 Refresh 按钮手动重读；弹窗不响应 WebSocket Run 事件。单个会话不可用时仍留在列表里并显示 "会话记录不可用"。worktree 已删的 Run 同样可用。

## Acceptance criteria

- [ ] 临时 `CURSOR_DATA_DIR` 下的 fixture 会话文件（slug 由 worktree 形式路径算出）能被接口读出；文件缺失、末行截断返回"不可用"（进程内 API 测试）
- [ ] 右侧按调用分段显示，点左侧调用会滚动定位
- [ ] 三类内容可分别开关；工具调用折叠；超过 60 条时可展开更早或全部
- [ ] token 用量与合计正确，未知用量时合计带 `≥`
- [ ] Refresh 能读到新追加的消息
- [ ] 前端 typecheck、lint、生产构建通过
