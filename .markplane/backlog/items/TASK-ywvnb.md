---
id: TASK-ywvnb
title: 创建类命令支持 --body-file
status: backlog
priority: high
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on: []
blocks:
- TASK-c6zs9
- TASK-kjdag
- TASK-c7x96
related: []
assignee: null
tags:
- ready-for-agent
position: a9
created: 2026-10-09
updated: 2026-10-09
---

# 创建类命令支持 --body-file

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

技能需要一步写入长正文。所有创建类命令（`add`、`epic`、`plan`、`note`）新增 `--body-file <path|->`，`-` 读 stdin；给定时用文件内容作为正文（标题行之后），不给时行为同上游（模板正文）。MCP `markplane_add` 同步支持传入正文。

## Acceptance criteria

- [ ] `add`/`epic`/`plan`/`note` 带 `--body-file f` 创建后，`show` 输出含文件正文
- [ ] `--body-file -` 从 stdin 读取
- [ ] 不带该参数时仍使用模板正文
- [ ] MCP `markplane_add` 传入正文后，生成文件与 CLI 一致
- [ ] CLI 集成测试与 MCP 对齐测试覆盖以上行为

## Blocked by

- 创建类命令支持 --body-file [[TASK-ywvnb]]
