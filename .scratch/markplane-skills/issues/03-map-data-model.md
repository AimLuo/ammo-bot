# 03: MAP 条目的数据模型

Type: grilling
Status: resolved
Blocked by: 01
Labels: wayfinder:grilling

## Question

MAP 作为 markplane 的新条目类型，应如何定义：ID 前缀（如 `MAP-xxxxx`）、存放目录、状态生命周期（例如 active / done / abandoned）、frontmatter 字段、正文各节（Destination / Notes / Decisions so far / Not yet specified / Out of scope）的约束，以及它与决策工单、最终产出的 spec（EPIC 或 PLAN）之间的关系？

## Answer

2026-10-09 与用户 grilling 确认（全部采纳推荐）：

1. **ID 与目录**：`MAP-xxxxx`（5 位随机后缀），存于 `.markplane/maps/items/` 与 `.markplane/maps/archive/`。
2. **状态**：写死的枚举 `active / done / abandoned`（同 EPIC/PLAN，不走可配置 workflow）；无 `draft`，map 建出即 `active`。`done` = 路线已清晰、无开放工单、可交接；`abandoned` = 放弃或目的地被重画。
3. **Destination 只在正文** `## Destination`；列表页 / `.context` 需要时截取该节首段，不设 frontmatter 字段。
4. **frontmatter**：`id, title, status, specs, related, tags, created, updated`；无 `priority`、无 `assignee`（claim 落在工单上）。
5. **MAP → Spec**：MAP 单向持有 `specs: [EPIC-…|PLAN-…]`，经 `link -r spec` 维护（目标仅限 EPIC/PLAN）；EPIC/PLAN 不加反向字段。目的地为纯决策时为空。
6. **MAP ↔ 工单**：关系只记在 TASK 侧（`Task.map`，单值，类比 `Task.epic`）；子工单靠查询得出，MAP 不存列表；一张工单只属于一张 map。字段细节归“决策工单在 TASK 上怎么表示”。
7. **正文结构**：模板预置 Destination / Notes / Decisions so far / Not yet specified / Out of scope 五节；core 提供按标题切节能力供页面与 `.context` 使用；`markplane check` 缺节仅告警。
8. **Decisions so far 条目格式**：`- 标题 [[TASK-xxxxx]]: gist`（不改 `[[ID]]` 语法）。
9. **完成后**：状态变为 `done`/`abandoned` 不触发自动动作；手动 `archive` 一张 map 时连带归档其全部子工单。

术语已写入根目录 `GLOSSARY.md`（Map、Destination、Spec）。
