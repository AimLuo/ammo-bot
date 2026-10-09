---
id: TASK-mbc9k
title: Web UI：Backlog 隐藏 Decision ticket 与 Task→Map 跳转
status: in-progress
priority: medium
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-2uakq
blocks:
- TASK-d3tdm
related: []
assignee: chuck
tags:
- ready-for-agent
position: aA
created: 2026-10-09
updated: 2026-10-09
---

# Web UI：Backlog 隐藏 Decision ticket 与 Task→Map 跳转

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

- Backlog 列表与看板默认隐藏 Decision ticket，提供显示开关。
- Task 详情显示 Map 字段，可跳转到 Map 路线图并高亮该节点。
- wiki-link chip 支持 `MAP` 前缀。

## Acceptance criteria

- [ ] 默认看不到 Decision ticket，打开开关后可见
- [ ] 从 Decision ticket 详情点击 Map 进入路线图且对应节点高亮
- [ ] 正文中的 `[[MAP-x]]` 渲染为可点击 chip
- [ ] `tsc`、lint、`npm run build` 通过

## Blocked by

- Web UI：/maps 列表与路线图详情页 [[TASK-2uakq]]
