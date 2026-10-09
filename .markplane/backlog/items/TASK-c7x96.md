---
id: TASK-c7x96
title: Map 条目类型
status: done
priority: high
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-ywvnb
blocks:
- TASK-kjetr
related: []
assignee: chuck
tags:
- ready-for-agent
position: aD
created: 2026-10-09
updated: 2026-10-09
---

# Map 条目类型

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

新增 **Map** 条目类型（见 `GLOSSARY.md`）。

- ID 前缀 `MAP`，目录 `.markplane/maps/`（`items/` + `archive/`）。
- 状态为写死枚举 `active / done / abandoned`，新建即 `active`；frontmatter：`id, title, status, specs, related, tags, created, updated`（无 priority、assignee）。
- `markplane map "<title>" [--body-file f]` 创建；`map` 模板预置 Destination / Notes / Decisions so far / Not yet specified / Out of scope 五节。
- core 提供按标题切节能力；`markplane check` 在 Map 缺节时告警（不报错）。
- `ls maps`、`show MAP-x`、`status MAP-x done` 可用；sync 处理 maps；wiki-link `[[MAP-x]]` 可解析。
- MCP `markplane_add` 支持 map 类型。

## Acceptance criteria

- [ ] `map "t"` 生成带五节模板的 Map；`--body-file` 时使用给定正文
- [ ] 非法状态被拒绝；`status MAP-x done|abandoned` 生效
- [ ] 缺节的 Map 在 `check` 中出现告警，退出码不因此失败
- [ ] `ls maps`、`show` 正常；`[[MAP-x]]` 引用不报 broken
- [ ] core 单元测试覆盖切节；CLI 集成测试与 MCP 对齐测试

## Blocked by

- 创建类命令支持 --body-file [[TASK-ywvnb]]
