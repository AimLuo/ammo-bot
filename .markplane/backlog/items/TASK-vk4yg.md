---
id: TASK-vk4yg
title: README、docs 与 CLAUDE.md 的改写范围
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-vs3wh
depends_on: []
blocks: []
related: []
assignee: null
tags: []
position: aR
created: 2026-10-09
updated: 2026-10-09
---

# README、docs 与 CLAUDE.md 的改写范围

## Question

README、`docs/`（architecture、file-format、cli-reference、mcp-setup、web-ui-guide、getting-started、ai-integration）、`CLAUDE.md` / `AGENTS.md`、`markplane claude-md` 输出片段，各自需要为 MAP、决策工单、frontier、新命令与 my-* 技能改写哪些内容？哪些上游文档保持不动？

## Answer

2026-10-09 与用户 grilling 确认（全部采纳推荐）：

1. **README**：顶部新增 fork 说明（相对上游新增了什么、mattpocock 技能工作流链路）；Installation 只保留源码安装（`cargo install --path crates/markplane-cli --features embed-ui`）并加 my-* 技能安装，删去 Homebrew / shell 脚本 / 预编译二进制；其余各节保持上游原样。
2. **`docs/`**（全部英文）：
   - 原地更新：`file-format`（MAP、`Task.map`、`Plan.tags`、`## Comments`、spec 模板）、`cli-reference`（新命令与参数）、`mcp-setup`（新 tools / resources）、`architecture`（数据模型加入 MAP）、`ai-integration`（`.context/maps.md`）、`web-ui-guide`（maps 页面）。
   - 新增 `docs/skills-workflow.md`：wayfinder → my-to-spec → to-tickets → implement-spec → code-review 在 markplane 上如何运作。
   - 不动：`getting-started`、`releasing`、`docs/archive/`、`docs/web-ui/` 设计稿。
3. **`CLAUDE.md` / `AGENTS.md`**：把根目录 `AGENTS.md` 中的个人工作区规则（主空间、`research/` / `docs/` / `.tmp/`、仅本地提交不 push）并入 `CLAUDE.md` 顶部，`AGENTS.md` 改为指向 `CLAUDE.md` 的软链；my-setup 的 `## Agent skills` 块与 markplane 的 `## Project Management` 片段都放在 `CLAUDE.md`。
4. **`markplane claude-md` 片段**：增加 `.markplane/maps/INDEX.md` 与 `.context/maps.md` 两行；标题保持 `## Project Management` 以维持幂等判断。
5. **自举**：在本仓库运行 `my-setup-matt-pocock-skills` 生成 `docs/agents/issue-tracker.md` 等，作为 spec 中的一张实现工单，排在“本 map 迁入 `.markplane`”之前。
