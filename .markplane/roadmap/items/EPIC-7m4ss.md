---
id: EPIC-7m4ss
title: Markplane 作为 mattpocock skills 的 tracker 后端
status: next
priority: high
started: null
target: null
related: []
tags:
- ready-for-agent
created: 2026-10-09
updated: 2026-10-09
---

# Markplane 作为 mattpocock skills 的 tracker 后端

## Problem Statement

我用 mattpocock skills（wayfinder、grill-with-docs、to-spec、to-tickets、implement、implement-spec、code-review）讨论需求、拆工单、实现任务。这些技能需要一个 issue tracker，但现有选项都不顺手：GitHub / GitLab 在仓库之外；本地 markdown tracker 只是一堆散文件，没有原生阻塞、没有查询、没有可视化，frontier 要靠人肉扫描。

markplane 把条目存成仓库里的 markdown，带 CLI、MCP 和 Web UI，几乎就是我想要的 tracker——但它不认识 wayfinder 的 Map 与决策工单，算不出 frontier，没有评论，创建时写不了正文，PLAN 不能脱离 TASK 独立存在，也没有一份能让这些技能直接使用的 tracker 文档。

## Solution

本仓库成为 markplane 的 fork（结构、命名与上游一致），并成为 mattpocock skills 的一等 tracker 后端：

- 新增 **Map** 条目类型、Web UI 的 `/maps` 列表页与路线图详情页，Decision ticket 作为挂在 Map 下的 Task。
- CLI / MCP 补齐技能需要的操作：带正文创建、frontier 查询、claim / resolve / comment、独立 PLAN、按 map / plan 过滤。
- `.context/` 摘要与 MCP resources 体现 active maps 与 frontier。
- 仓库根目录提供两个改造技能：`my-to-spec`（询问发布成 EPIC 还是 PLAN）与 `my-setup-matt-pocock-skills`（新增 Markplane tracker 选项及其 tracker 文档模板）。其余技能原样使用上游。
- 本仓库自己用 markplane 作为 tracker（自举），并把规划本 spec 的 wayfinder map 迁入 `.markplane`。

## User Stories

### Map 与 Decision ticket

1. As a developer, I want to create a Map with a full body in one command, so that wayfinder can chart a map without a create-then-edit dance.
2. As a developer, I want each Map to carry `active` / `done` / `abandoned` status, so that I can tell live efforts from finished or dropped ones.
3. As a developer, I want a new Map's body to be pre-filled with Destination / Notes / Decisions so far / Not yet specified / Out of scope sections, so that every map has the shape wayfinder expects.
4. As a developer, I want `markplane check` to warn when a Map body is missing one of those sections, so that malformed maps are noticed without blocking work.
5. As a developer, I want to create a Decision ticket by adding a Task with `--map` and a wayfinder type (research / prototype / grilling / task), so that decision work lives in the same tracker as implementation work.
6. As a developer, I want a Decision ticket's type to be validated against the four built-in wayfinder types instead of my configured task types, so that maps work in every repo without config edits.
7. As a developer, I want a Task to be rejected if it sets `map` together with `epic` or `plan`, so that decision tickets never masquerade as implementation work.
8. As a developer, I want to re-parent a Decision ticket to another Map by linking it again, so that mis-filed tickets are easy to fix.
9. As a developer, I want to wire blocking edges between Decision tickets with the existing `link -r blocks`, so that the frontier is computed from native dependencies.
10. As a wayfinder session, I want `ls --map <MAP> --frontier` to return the open, unblocked, unassigned tickets of a Map in `position` order, so that I can take the first frontier ticket deterministically.
11. As a wayfinder session, I want a blocker that was cancelled to count as resolved, so that tickets ruled out of scope don't strand their dependents.
12. As a wayfinder session, I want to claim a ticket with `start --user`, so that concurrent sessions skip it.
13. As a wayfinder session, I want `resolve <TASK> --answer-file f --gist "…"` to write the `## Answer` section, mark the ticket done, and append `- Title [[ID]]: gist` to the Map's Decisions so far, so that recording a resolution is one atomic step.
14. As a wayfinder session, I want `resolve` to tell me when the Map has no open tickets left, so that I can ask the human whether to mark the Map done.
15. As a developer, I want to rule a ticket out of scope by cancelling it, so that it leaves the frontier without being recorded as a decision.
16. As a developer, I want archiving a Map to archive all its Decision tickets too, so that finished efforts leave no orphans in the backlog.
17. As a developer, I want a Map to record which Specs it produced via `link <MAP> -r spec`, so that I can trace a spec back to the decisions behind it.

### Spec 与 Implementation ticket

18. As a developer running my-to-spec, I want to be asked whether to publish as an EPIC or a PLAN, with a recommendation based on the conversation, so that user-facing features land on the roadmap and technical plans don't.
19. As a developer, I want EPIC and PLAN specs to share the same spec template (Problem Statement … Further Notes), so that downstream skills read them the same way.
20. As a developer, I want to create a standalone PLAN without a parent Task, so that a PLAN can be a spec with tickets under it.
21. As a developer, I want PLANs to have tags, so that `ready-for-agent` works on every spec kind.
22. As a developer, I want EPIC specs published as `next` and PLAN specs as `approved`, so that the spec's readiness is visible in its status.
23. As to-tickets, I want to attach Implementation tickets with `add --epic` or `add --plan`, so that the spec's task graph is native.
24. As implement-spec, I want `ls --epic X` / `ls --plan X` (optionally with `--frontier`) to list a spec's tickets, so that I can dispatch the frontier of implementation work.
25. As implement-spec, I want the tracker doc to tell me that starting a spec moves it to `now` / `in-progress` and finishing moves it to `done`, so that closing work needs no PRs.
26. As code-review, I want commits to carry a `Refs: TASK-xxxxx` trailer, so that I can fetch the originating ticket and spec with `markplane show`.
27. As my-to-spec, I want to link the published spec back to the Map the conversation came from, so that the map–spec relation is recorded automatically.

### Comments 与正文

28. As any skill, I want `--body-file <path|->` on create commands, so that long bodies are written in one step.
29. As any skill, I want `comment <ID> --body-file f` to append a dated, attributed entry under `## Comments`, so that discussion has a home on every item kind.
30. As to-tickets, I want `show` to return comments as part of the item, so that I can read a spec's full discussion.

### Web UI

31. As a developer, I want a `/maps` list page whose cards show ID, status, title, progress (decided / total) and frontier count, so that I can see every effort at a glance.
32. As a developer, I want a Map detail page that lays Decision tickets out as a left-to-right route graph by dependency depth, from Start to the Destination, so that I can see the shape of the way ahead.
33. As a developer, I want node borders to encode decided / claimed / frontier / blocked state and nodes to show type badge and title, so that the frontier is visible without reading every ticket.
34. As a developer, I want clicking a node to show its Question, Answer and blockers in a side panel, with Not yet specified and Out of scope beneath, so that I can zoom into any decision.
35. As a developer, I want the Backlog board to hide Decision tickets by default with a toggle to show them, so that implementation work isn't drowned out.
36. As a developer, I want a Task's detail to show its Map and jump to the route graph with that node highlighted, so that I can get back from a ticket to its map.

### AI context

37. As an agent reading `.context/summary.md`, I want an Active Maps section (at most 3 maps, each with progress and the next frontier ticket), so that I know which efforts are in flight.
38. As an agent, I want a `.context/maps.md` with each active map's destination, frontier, claimed tickets, blocked count and fog count, so that I can pick up a map without loading every ticket.
39. As an agent, I want Decision tickets excluded from the Priority Queue, metrics and blocked items, but claimed ones shown in active work with their map, so that the summaries stay about implementation while still showing who is deciding what.
40. As an MCP client, I want `markplane://maps` and `markplane://map/{id}` resources, so that I can read maps the same way as other items.
41. As an MCP client, I want MCP tools at parity with the CLI (body on add, map / plan kinds, map / plan / frontier filters, comment, resolve, richer query results), so that MCP-only agents aren't second-class.

### Skills 与分发

42. As a developer setting up a repo, I want my-setup-matt-pocock-skills to offer Markplane (recommended when `.markplane/` exists) and write `docs/agents/issue-tracker.md` from its Markplane template, so that every skill knows how to use markplane.
43. As a developer, I want that tracker doc to describe only CLI commands, so that skills never hesitate between two interfaces.
44. As another user of this fork, I want `scripts/link-skills.sh` to symlink the repo's `my-*` skills into my agents' skill directories, so that installing them is one command.
45. As the owner, I want to import the `my-*` skills through skills-manager with the `my` tag, so that they deploy like my other skills.

### 仓库自身

46. As the owner, I want upstream's own `.markplane` items removed while config and templates stay, so that this repo's context describes this project.
47. As the owner, I want this repo configured with my-setup and using markplane as its own tracker, so that the fork is dogfooded.
48. As the owner, I want the wayfinder map that planned this spec migrated into `.markplane` as a Map with its Decision tickets, answers and blocking edges, linked to this EPIC, so that the route graph shows real data and `.scratch` can be deleted.
49. As the owner, I want an `upstream` remote whose history is tied in with a one-time `-s ours` merge of the imported commit, so that future upstream merges are normal three-way merges.
50. As a developer, I want `markplane --version` to show a `+skills.N` build suffix, so that the fork's binary is distinguishable from upstream's.
51. As a reader, I want the README to explain what this fork adds, the skills workflow, and source-only installation, so that I can get started without reading the map.
52. As a reader, I want `docs/skills-workflow.md` to walk through wayfinder → my-to-spec → to-tickets → implement-spec → code-review on markplane, so that the whole flow is documented in one place.

## Implementation Decisions

### Domain model（core）

- **Map**：ID 前缀 `MAP`（5 位随机后缀），目录 `maps/`（`items/` + `archive/`）。状态为写死的枚举 `active / done / abandoned`，新建即 `active`。frontmatter：`id, title, status, specs, related, tags, created, updated`（无 priority、无 assignee）。Destination 只写在正文，不设 frontmatter 字段。
- **Map 正文**：模板预置五节；core 提供按标题切节能力（供 UI 与 `.context` 使用）；`check` 缺节仅告警。
- **Map → Spec**：Map 单向持有 `specs`（只能指向 EPIC / PLAN），经 `link -r spec` 维护；EPIC / PLAN 不加反向字段。
- **Task.map**：单值 `Option<MAP-id>`，经 `link -r map` 设置；关系只记在 Task 侧，Map 的子工单靠查询得出。有 `map` 即 Decision ticket；与 `epic` / `plan` 互斥，由 core 校验。
- **Decision ticket 类型**：复用 `Task.type`；有 `map` 时 `type` 必须是内置的 `research / prototype / grilling / task`，并跳过 `task_types` 配置校验。`config.task_types` 不新增这几个值。
- **状态语义**（按 StatusCategory，不写死状态名）：新建 → backlog；claim = assignee 非空（`start --user` 同时进入 active）；resolve → completed；范围外 → cancelled。
- **阻塞判定（全局）**：`find_blocked_items` 等逻辑把 completed 与 cancelled 的 blocker 都视为已解除。
- **Frontier**：父范围内（map / epic / plan）open、所有 `depends_on` 已关闭、assignee 为空的 Task，按 `position` 排序；决策工单统一默认 priority。
- **Plan**：新增 `tags`；允许不挂 Task 独立创建。
- **评论**：所有条目正文末尾 `## Comments`，每条为 `### <日期> <作者>` 小节。Decision ticket 的结论独立写在 `## Answer`。
- **归档**：手动 `archive` 一张 Map 连带归档其全部 Decision ticket；Map 状态变化不触发自动动作。
- **spec 模板**：新增 `spec` 模板（to-spec 的七节结构），在 manifest 中同时注册到 epic 与 plan；新增 `map` 模板。

### CLI（tracker 文档以此为准）

- 新命令：`markplane map "<title>" [--body-file f]`；`markplane comment <ID> --body-file f`；`markplane resolve <ID> --answer-file f --gist "<一句话>"`（原子地写 `## Answer`、置 done、在所属 Map 的 Decisions so far 追加 `- 标题 [[ID]]: gist`；若 Map 已无 open 工单则打印提示）。
- 扩展：创建类命令加 `--body-file <path|->`；`add` 加 `--map`、`--plan`；`ls` 加 `--map`、`--plan`、`--frontier`（可与 `--epic` / `--plan` / `--map` 组合）；`plan [TASK_ID] [--title t] [--body-file f] [--tags …]`（带 TASK_ID 行为同上游）；`link` 新增关系 `map`、`spec`；`epic` 支持 `--tags`。
- 局部修改（如 Map 某一节、Out of scope）继续直接编辑文件，不做按节命令。
- `markplane claude-md` 片段增加 `.markplane/maps/INDEX.md` 与 `.context/maps.md` 两行，标题保持 `## Project Management`。

### MCP

- 与 CLI 对齐：`markplane_add` 支持正文及 map / plan 类型；`markplane_query` 支持 map / plan / frontier 过滤，返回补 assignee / depends_on / tags；新增 `markplane_comment`、`markplane_resolve`。
- Resources：新增 `markplane://maps`（内容即 `.context/maps.md`）与 URI 模板 `markplane://map/{id}`。`markplane_context` 不加 map focus。

### Context layer

- `summary.md`：Now 之后新增 `## Active Maps`，每张一行 `MAP-x: 标题 — 已决策/总数, next: <首张 frontier 标题> (TASK-id)`，最多 3 张。
- 新增 `.context/maps.md`（每张 active map 一节：Destination 首段、frontier、已 claim 工单与负责人、被阻塞数、Not yet specified 条目数；约 300–500 token，独立于 summary 预算）与 `maps/INDEX.md`。
- Decision ticket 不进 Priority Queue、Backlog 计数、metrics、blocked-items；已 claim 的出现在 active-work 并标注 Map。

### Web UI

- 新增 `/maps` 列表页与 Map 详情页（路线图），交互如 User Stories 31–36 所述；被 cancelled 的工单不进图，列在 Out of scope。
- Backlog / 看板默认隐藏 Decision ticket，提供开关；Task 详情显示 Map 字段并可跳转、高亮节点；wiki-link chip 支持 `MAP` 前缀。
- 以下片段来自原型（分支 `prototype/map-page`，Variant C），记录节点状态的判定：

```ts
type TicketState = "decided" | "claimed" | "frontier" | "blocked" | "out-of-scope";
// cancelled → out-of-scope; done → decided; assignee → claimed;
// any dependsOn not closed (done|cancelled) → blocked; else frontier
// layout: depth = 0 if no deps, else 1 + max(depth of deps); column = depth
```

### Skills（仓库根目录）

- 只做两个 fork，各含 `SKILL.md` 与 `agents/openai.yaml`，frontmatter `name: my-*`，description 末尾注明 “Based on mattpocock/skills v1.3.1”：
  - **my-to-spec**：保留“不访谈”，只多问一次 EPIC 还是 PLAN 并附推荐（依据：EPIC = 面向用户的功能 / 成果；PLAN = 偏技术的方案，可从属已有 EPIC）；seams 确认保留；按选择执行 `markplane epic` / `markplane plan --title …`，带 `--body-file` 与 `--tags ready-for-agent`；对话来自 Map 时发布后 `link <MAP> -r spec`，不改 Map 状态。
  - **my-setup-matt-pocock-skills**：以上游为底，只新增 Markplane 选项（探测到 `.markplane/` 时优先推荐）；携带 `issue-tracker-markplane.md` 模板并写入目标仓库 `docs/agents/issue-tracker.md`；`CLAUDE.md` 中 `## Agent skills` 块与 markplane 的 `## Project Management` 片段并存。
- **tracker 文档内容**：只写 CLI；覆盖 Conventions（建、读、查、挂 parent、评论、标签、关闭、编辑、删除 = cancelled + archive）、publish / fetch 映射、“how this tracker closes work”（EPIC next → now → done；PLAN approved → in-progress → done；不走 PR）、commit `Refs:` trailer、triage 角色映射为同名 tag（Plan 也有 tags）、to-tickets 挂 PLAN 时 `Plan.implements` 的自动维护属关系簿记而非修改父 issue、Wayfinding operations（Map / Child / Blocking / Frontier / Claim / Resolve / Map done 需询问用户后 `status MAP done` / 按名称引用 `标题 [[ID]]`）、Decision ticket 永不带 `ready-for-agent`、research 产出仍放 `research/<name>` 分支。
- **分发**：`scripts/link-skills.sh`（参照上游脚本）把根目录 `my-*` 软链到 `~/.claude/skills`、`~/.agents/skills`、`~/.cursor/skills`；所有者本人用 skills-manager 导入并打 `my` 标签。上游未改动的技能不随仓库分发。

### 仓库与上游

- 删除 `.markplane/` 下上游的全部条目与归档（含 notes 的 `decisions.md`、`ideas.md`），保留 `config.yaml` 与 `templates/`；`config.yaml` 的 description 改为说明本 fork 的定位。
- 添加 `upstream` remote 并一次性 `git merge -s ours --allow-unrelated-histories 73cce05`；之后按需三方合并，`.markplane/` 条目冲突取删除，新功能冲突以本 fork 为准，合并后 test / clippy / UI build 全绿才提交。
- 版本号改为 `0.1.4+skills.1`。`release.yml`、Homebrew 相关、`install.sh` 原样保留不动；只支持源码安装。
- 文档（英文）：README 顶部加 fork 说明与工作流，Installation 只留源码安装与 my-* 技能安装；原地更新 `file-format`、`cli-reference`、`mcp-setup`、`architecture`、`ai-integration`、`web-ui-guide`；新增 `docs/skills-workflow.md`；`getting-started`、`releasing`、`docs/archive/`、`docs/web-ui/` 不动。
- 根目录 `AGENTS.md` 的个人工作区规则并入 `CLAUDE.md` 顶部，`AGENTS.md` 改为指向 `CLAUDE.md` 的软链。`GLOSSARY.md` 保留。

### 顺序约束

- 本仓库运行 my-setup 的工单排在“把本 map 迁入 `.markplane`”之前；迁移工单依赖 Map / Decision ticket / resolve 等功能落地，迁移后删除 `.scratch/markplane-skills/`，保留 `research/*`、`prototype/*` 分支。

## Testing Decisions

- **好测试的标准**：只测外部行为——命令输出、生成文件的 frontmatter 与正文——不测内部实现。实现时按 tdd 在下列 seam 上先写测试。
- **主 seam：CLI 集成测试**（先例：`assert_cmd` + 临时目录 `init --empty` 的现有 CLI 集成测试）。必须覆盖：`map` 带正文创建；`add --map` 类型校验与 epic / plan 互斥；`ls --frontier` 过滤与 `position` 排序（含 cancelled blocker 视为解除）；`resolve` 的三个效果；`comment`；独立 `plan` 与 `--plan`；`link -r map` / `-r spec`；归档 Map 连带工单；`Plan.tags`。
- **MCP 集成测试**（先例：现有 MCP 集成测试）：每个新增 / 修改的 tool 与 resource 一组对齐测试，验证与 CLI 行为一致，不重复 CLI 已覆盖的细节。
- **core 单元测试**（先例：project 与 context 模块内的 tests）：只覆盖纯逻辑——frontier 计算、Map 正文切节、阻塞判定的 cancelled 处理、`.context` 生成（Active Maps、`maps.md`、Decision ticket 的排除规则）。
- **Web UI**：不引入前端测试框架；`tsc` + lint + build 兜底，用迁入的本 map 真实数据人工核对 `/maps` 与路线图。
- **my-* 技能**：不写自动化测试，以自举为验收——本仓库运行 my-setup 后 `docs/agents/issue-tracker.md` 可直接使用；本 map 迁入后用 wayfinder 在 markplane 上完成一次 claim + resolve；用 my-to-spec 发布一份小 spec 并确认它询问 EPIC 还是 PLAN。
- CI 现有门槛不变：`cargo fmt --check`、`clippy -D warnings`、`cargo test`、前端构建。

## Out of Scope

- 产品改名（继续叫 markplane）。
- 旧 ammo-bot 的 scanner / pipeline / PM2 / bun 等全部内容，以及 Beads tracker 支持。
- 与 tracker 无关的 mattpocock 技能（tdd、teach 等）的改造；除 my-to-spec 与 my-setup-matt-pocock-skills 之外的任何技能 fork。
- 发布渠道（GitHub Releases、Homebrew、`install.sh`）的改造或启用。
- 前端测试框架。
- `markplane_context` 的 map focus；按节编辑 Map 的专用命令；真正的 delete 命令（以 cancelled + archive 代替）。

## Further Notes

- 本 spec 由 wayfinder map「以 markplane 为底适配 mattpocock skills」规划而来（当前在 `.scratch/markplane-skills/map.md`，13 张决策工单的完整结论见其 `issues/`）。迁入 `.markplane` 后应 `link <MAP> -r spec` 指向本 EPIC。
- 调研原始材料：分支 `research/skill-tracker-deps`；页面原型：分支 `prototype/map-page`（选定 Variant C）。
- 术语见根目录 `GLOSSARY.md`：Map、Destination、Decision ticket、Frontier、Spec、Implementation ticket。
- 上游基线：`zerowand01/markplane` commit `73cce05`；mattpocock/skills v1.3.1。
- 实现期检查项：skills-manager 导入仓库内 `my-*` 后，修改仓库中的技能能否自动生效。
