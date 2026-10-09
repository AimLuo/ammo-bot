---
id: TASK-c6zs9
title: 评论：comment 命令与 show 展示评论
status: done
priority: high
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-ywvnb
blocks:
- TASK-k4tfw
related: []
assignee: chuck
tags:
- ready-for-agent
position: aA
created: 2026-10-09
updated: 2026-10-09
---

# 评论：comment 命令与 show 展示评论

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

所有条目类型都能承载讨论。`markplane comment <ID> --body-file f` 在正文末尾 `## Comments` 节（不存在则创建）下追加 `### <日期> <作者>` 小节；`show` 返回的内容包含评论。新增 MCP 工具 `markplane_comment`。

## Acceptance criteria

- [ ] 对 TASK / EPIC / PLAN / NOTE 执行 `comment` 均在 `## Comments` 下追加带日期与作者的小节，多次评论按时间顺序累加
- [ ] `show <ID>` 输出包含评论
- [ ] MCP `markplane_comment` 产生与 CLI 相同的文件内容
- [ ] CLI 集成测试与 MCP 对齐测试

## Blocked by

- 创建类命令支持 --body-file [[TASK-ywvnb]]
