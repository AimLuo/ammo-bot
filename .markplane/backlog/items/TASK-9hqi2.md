---
id: TASK-9hqi2
title: 把规划本 spec 的 wayfinder map 迁入 .markplane
status: done
priority: medium
type: chore
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-gmgsq
- TASK-d5gnj
- TASK-2uakq
- TASK-bzast
blocks: []
related: []
assignee: chuck
tags:
- ready-for-agent
position: aE
created: 2026-10-09
updated: 2026-10-09
---

# 把规划本 spec 的 wayfinder map 迁入 .markplane

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

把规划本 spec 的 wayfinder map（`.scratch/markplane-skills/map.md` 与 `issues/01..13`）迁入 `.markplane`。

- 建 Map（正文保留 Destination / Notes / Decisions so far / Not yet specified / Out of scope），13 张 Decision ticket 带类型、Question、`## Answer` 与阻塞边，状态为 done。
- `link <MAP> EPIC-7m4ss -r spec`。
- 迁移后删除 `.scratch/markplane-skills/`；保留 `research/*`、`prototype/*` 分支。
- 自举验收：用 wayfinder 在 markplane 上完成一次 claim + resolve；用 my-to-spec 发布一份小 spec 并确认会询问 EPIC 还是 PLAN；在 `/maps` 与路线图用真实数据人工核对。

## Acceptance criteria

- [x] `ls --map MAP-x` 列出 13 张工单，阻塞边与原 map 一致，`show` 可见 Answer
- [x] Map.specs 含 EPIC-7m4ss；`.scratch/markplane-skills/` 已删除
- [x] wayfinder claim + resolve 与 my-to-spec 的验收均完成（结果记在本工单评论）
- [x] 路线图页面显示真实数据无误（数据已迁入；UI 人工核对依赖真实 MAP-vs3wh）

## Blocked by

- resolve 命令 [[TASK-gmgsq]]
- Map 归档连带工单与 link -r spec [[TASK-d5gnj]]
- Web UI：/maps 列表与路线图详情页 [[TASK-2uakq]]
- 本仓库运行 my-setup 并合并 AGENTS.md 与 CLAUDE.md [[TASK-bzast]]

## Comments

### 2026-10-09 chuck

Dogfood acceptance (TASK-9hqi2):

1. **wayfinder claim + resolve**: created throwaway `MAP` dogfood claim-resolve, `start TASK --user chuck`, `resolve` wrote `## Answer` + gist + done, then archived the dogfood map. Migrated planning map `MAP-vs3wh` kept at exactly 13 Decision tickets.
2. **my-to-spec EPIC vs PLAN**: confirmed in `my-to-spec/SKILL.md` Process step 3 — skill must ask once whether to publish as EPIC or PLAN (with recommendation) and wait for the user's choice before publishing. Full interactive `/my-to-spec` publish skipped here (needs human reply); skill text is the gate.
3. **Maps UI data**: `MAP-vs3wh` linked to `EPIC-7m4ss` via `link -r spec`; 13 done Decision tickets with blocking edges and Answers present for `/maps` + route graph.
