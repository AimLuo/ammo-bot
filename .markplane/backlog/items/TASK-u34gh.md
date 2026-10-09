---
id: TASK-u34gh
title: my-to-spec 技能
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on:
- TASK-kjdag
- TASK-d5gnj
blocks: []
related: []
assignee: chuck
tags:
- ready-for-agent
position: aB
created: 2026-10-09
updated: 2026-10-09
---

# my-to-spec 技能

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

在仓库根目录新增 `my-to-spec` 技能（`SKILL.md` + `agents/openai.yaml`，frontmatter `name: my-to-spec`，description 末尾注明 “Based on mattpocock/skills v1.3.1”），以上游 to-spec 为底：

- 保留“不访谈”，只多问一次发布成 EPIC 还是 PLAN 并附推荐（EPIC = 面向用户的功能 / 成果；PLAN = 偏技术的方案，可从属已有 EPIC）；保留 seams 确认。
- 按选择执行 `markplane epic` / `markplane plan --title …`，带 `--body-file` 与 `--tags ready-for-agent`；EPIC 发布为 `next`，PLAN 为 `approved`。
- 对话来自 Map 时，发布后 `link <MAP> -r spec`，不改 Map 状态。

编写时参考 `writing-for-agents`。

## Acceptance criteria

- [ ] 技能文件结构与 frontmatter 符合约定，与上游差异仅限上述几点
- [ ] 按技能文本手动走一遍：EPIC 与 PLAN 两条路径的命令均能在当前 CLI 成功执行，状态与 tags 正确
- [ ] 来自 Map 的路径正确写入 Map.specs

## Blocked by

- 独立 PLAN、Plan tags 与 spec 模板 [[TASK-kjdag]]
- Map 归档连带工单与 link -r spec [[TASK-d5gnj]]
