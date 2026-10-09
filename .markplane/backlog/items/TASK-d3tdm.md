---
id: TASK-d3tdm
title: 文档：README、docs 更新与 skills-workflow.md
status: done
priority: medium
type: chore
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-z3uwm
- TASK-mbc9k
- TASK-k4tfw
blocks: []
related: []
assignee: chuck
tags:
- ready-for-agent
position: aF
created: 2026-10-09
updated: 2026-10-09
---

# 文档：README、docs 更新与 skills-workflow.md

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

英文文档更新：

- README 顶部加 fork 说明（新增了什么、skills 工作流），Installation 只留源码安装与 `my-*` 技能安装。
- 原地更新 `file-format`、`cli-reference`、`mcp-setup`、`architecture`、`ai-integration`、`web-ui-guide`，覆盖 Map、Decision ticket、新命令、MCP 工具与资源、context 文件、新页面。
- 新增 `docs/skills-workflow.md`：wayfinder → my-to-spec → to-tickets → implement-spec → code-review 在 markplane 上的完整流程。
- `getting-started`、`releasing`、`docs/archive/`、`docs/web-ui/` 不动。

## Acceptance criteria

- [x] 文档中的命令与当前 CLI 一致（抽查可执行）
- [x] README 只保留源码安装
- [x] `docs/skills-workflow.md` 覆盖五个技能的衔接

## Blocked by

- Context 层与 MCP map resources [[TASK-z3uwm]]
- Web UI：Backlog 隐藏 Decision ticket 与 Task→Map 跳转 [[TASK-mbc9k]]
- my-setup-matt-pocock-skills、Markplane tracker 文档与 link-skills.sh [[TASK-k4tfw]]
