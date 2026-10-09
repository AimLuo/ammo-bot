---
id: TASK-bzast
title: 本仓库运行 my-setup 并合并 AGENTS.md 与 CLAUDE.md
status: done
priority: medium
type: chore
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-pd65d
- TASK-k4tfw
blocks:
- TASK-9hqi2
related: []
assignee: chuck
tags:
- ready-for-agent
position: aD
created: 2026-10-09
updated: 2026-10-09
---

# 本仓库运行 my-setup 并合并 AGENTS.md 与 CLAUDE.md

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

自举：本仓库用 my-setup 配置 markplane 作为 tracker。

- 运行 `my-setup-matt-pocock-skills`，生成 `docs/agents/issue-tracker.md` 与 `CLAUDE.md` 的 `## Agent skills` 块。
- 根目录 `AGENTS.md` 的个人工作区规则并入 `CLAUDE.md` 顶部，`AGENTS.md` 改为指向 `CLAUDE.md` 的软链。`GLOSSARY.md` 保留。

## Acceptance criteria

- [x] `docs/agents/issue-tracker.md` 存在且可直接使用
- [x] `CLAUDE.md` 同时含工作区规则、`## Agent skills` 与 `## Project Management`
- [x] `AGENTS.md` 是指向 `CLAUDE.md` 的软链

## Blocked by

- 仓库接上游并清理上游 .markplane 数据 [[TASK-pd65d]]
- my-setup-matt-pocock-skills、Markplane tracker 文档与 link-skills.sh [[TASK-k4tfw]]
