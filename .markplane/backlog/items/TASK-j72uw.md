---
id: TASK-j72uw
title: my-* 技能清单和分发安装方式
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-vs3wh
depends_on:
- TASK-3mkf9
- TASK-9yxpq
blocks: []
related: []
assignee: null
tags: []
position: aN
created: 2026-10-09
updated: 2026-10-09
---

# my-* 技能清单和分发安装方式

## Question

最终需要哪些 `my-*` 技能（放在仓库根目录 `my-*/SKILL.md`），每个相对上游改了什么；它们与未改动的上游技能如何一起安装到 Cursor / Claude Code 等 agent（软链、脚本、plugin），以及 `my-setup-matt-pocock-skills`（若需要）如何写出 markplane 的 tracker 文档？

## Answer

2026-10-09 与用户 grilling 确认（全部采纳推荐）：

1. **清单**：只做 `my-to-spec` 与 `my-setup-matt-pocock-skills`；其余技能（含 wayfinder、to-tickets、implement-spec）靠 tracker 文档原样使用。
2. **`my-setup-matt-pocock-skills` 以上游为底**，只新增 Markplane 一个 tracker 选项；装到用户本机后替换现有的 Beads 版本（Beads 支持随旧 ammo-bot 一起丢弃）。
3. **安装**：仓库提供 `scripts/link-skills.sh`（参照上游同名脚本），把根目录 `my-*` 软链到 `~/.claude/skills`、`~/.agents/skills`、`~/.cursor/skills`，供其他使用者；用户本人用 skills-manager 从仓库根目录导入 `my-*`、打 `my` 标签并部署。实现期检查项：skills-manager 导入后，仓库内修改技能能否自动生效。
4. **未改动的上游技能**不随仓库分发；README 写明需先安装 mattpocock/skills。上游文本中的 “run `/setup-matt-pocock-skills`” 在 tracker 文档存在时不会触发，不改。
5. **tracker 模板** `issue-tracker-markplane.md` 只放在 `my-setup-matt-pocock-skills/` 目录内，由它写入目标仓库 `docs/agents/issue-tracker.md`；`CLAUDE.md` 的 `## Agent skills` 块与 `markplane claude-md` 片段并存；探测到 `.markplane/` 时优先推荐 Markplane。
6. **`my-to-spec` 相对上游的改动**：保留“不访谈”，只多问一次“EPIC 还是 PLAN”并附推荐（seams 确认保留）；按选择执行 `markplane epic` 或 `markplane plan --title …`，带 `--body-file` 与 `--tags ready-for-agent`；对话来自某张 map 时自动 `link <MAP> -r spec`。
7. **目录格式**：每个 `my-*/` 含 `SKILL.md` 与 `agents/openai.yaml`；frontmatter `name: my-*`，description 末尾注明 “Based on mattpocock/skills v1.3.1”。
