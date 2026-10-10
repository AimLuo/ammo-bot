---
id: TASK-h2vcn
title: 详情面板分开描述与评论，并能发评论
status: planned
priority: medium
type: feature
effort: large
epic: EPIC-24iwg
plan: null
depends_on:
- TASK-hs89w
blocks:
- TASK-hreg4
- TASK-55iug
related: []
assignee: null
tags:
- ready-for-agent
position: ax
created: 2026-10-10
updated: 2026-10-10
---

# 详情面板分开描述与评论，并能发评论

## Parent

任务卡片评论、Needs You 与会话记录 [[EPIC-24iwg]]

## What to build

人在 Task 详情面板里读评论、写评论；保存描述不再覆盖 agent 刚追加的评论。

- core：把正文解析成 `description`（去掉评论段）和评论列表 `{date, author, body}`，规则见父 Spec「评论模型」（`### {日期} {作者}`、首条评论前的文字保留不列出、围栏内标题不参与解析）。新增"只替换描述"的写入：在条目锁内替换非评论部分，把磁盘上当前的评论段原样接回文末。
- API：`POST /api/tasks/{id}/comments`（`{ "text" }`，去空白后为空 400，只接受 Task ID，作者取 serve 进程默认作者、客户端不能指定，返回 201 和更新后的 Task）；Task 响应增加 `description` 和 `comments`，`body` 保持兼容；`PATCH /api/tasks/{id}` 增加 `description` 字段。
- 详情面板：元数据 → 描述（编辑器只编辑描述）→ "Comments (n)" 正序列表（作者、日期、Markdown 渲染、wiki-link 可用；作者是 `dispatcher` 或 `agent-*` 加机器人图标）→ 底部评论框。已归档 Task 只读、无评论框。
- 可复用的 `CommentComposer`：现有 textarea 组件、主按钮 "Comment"、Cmd/Ctrl+Enter 提交、空内容或请求中禁用、成功清空、失败保留文字并 toast；Props `onSubmit(text)`、`submitLabel`、`extraActions`（附加按钮插槽）、`disabled`。
- 卡片在有评论时显示评论图标和数量。评论变化沿用现有文件变化 WebSocket 事件自动出现在打开的面板里。

## Acceptance criteria

- [ ] 在详情面板发评论后，文件末尾追加了以 serve 默认作者署名的评论，评论框清空
- [ ] 客户端读取之后文件又被追加了评论，此时 PATCH `description` 仍保留那条评论（进程内 API 测试）
- [ ] 空文本返回 400；非 Task ID 被拒绝；评论中的标题被降级（API 测试）
- [ ] 机器写的评论带机器人图标；归档 Task 不显示评论框
- [ ] 卡片显示评论数；CLI 追加的评论无需刷新即出现在已打开的面板
- [ ] 前端 typecheck、lint、生产构建通过
