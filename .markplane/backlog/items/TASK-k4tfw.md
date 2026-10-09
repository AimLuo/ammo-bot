---
id: TASK-k4tfw
title: my-setup-matt-pocock-skills、Markplane tracker 文档与 link-skills.sh
status: in-progress
priority: medium
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-c6zs9
- TASK-62jjn
- TASK-gmgsq
- TASK-d5gnj
blocks:
- TASK-bzast
- TASK-d3tdm
related: []
assignee: chuck
tags:
- ready-for-agent
position: aC
created: 2026-10-09
updated: 2026-10-09
---

# my-setup-matt-pocock-skills、Markplane tracker 文档与 link-skills.sh

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

在仓库根目录新增 `my-setup-matt-pocock-skills`（以上游为底），并提供分发脚本。

- 只新增 Markplane tracker 选项，探测到 `.markplane/` 时优先推荐；写入目标仓库 `docs/agents/issue-tracker.md`（来自技能内 `issue-tracker-markplane.md` 模板）；`CLAUDE.md` 中 `## Agent skills` 块与 markplane 的 `## Project Management` 片段并存。
- tracker 文档**只写 CLI**，覆盖：Conventions（建、读、查、挂 parent、评论、标签、关闭、编辑、删除 = cancelled + archive）；publish / fetch 映射；how this tracker closes work（EPIC next → now → done；PLAN approved → in-progress → done；不走 PR）；commit `Refs: TASK-xxxxx` trailer；triage 角色映射为同名 tag；to-tickets 挂 PLAN 时 `Plan.implements` 自动维护属簿记而非修改父 issue；Wayfinding operations（Map / Child / Blocking / Frontier / Claim / Resolve / Map done 需询问用户后 `status MAP done` / 按名称引用 `标题 [[ID]]`）；Decision ticket 永不带 `ready-for-agent`；research 产出放 `research/<name>` 分支。
- `scripts/link-skills.sh`（参照上游脚本）把根目录 `my-*` 软链到 `~/.claude/skills`、`~/.agents/skills`、`~/.cursor/skills`。
- 所有者用 skills-manager 导入并打 `my` 标签，替换现有 Beads 版本 `my-setup-matt-pocock-skills`；顺带确认修改仓库中的技能能否自动生效。

## Acceptance criteria

- [ ] tracker 文档中每条命令都能在当前 CLI 执行（逐条手动验证）
- [ ] `scripts/link-skills.sh` 可重复执行，正确建立软链
- [ ] skills-manager 中 `my-setup-matt-pocock-skills` 为本仓库版本并带 `my` 标签
- [ ] frontmatter 与 description 符合约定

## Blocked by

- 评论：comment 命令与 show 展示评论 [[TASK-c6zs9]]
- Frontier 查询与 cancelled blocker 视为解除 [[TASK-62jjn]]
- resolve 命令 [[TASK-gmgsq]]
- Map 归档连带工单与 link -r spec [[TASK-d5gnj]]
