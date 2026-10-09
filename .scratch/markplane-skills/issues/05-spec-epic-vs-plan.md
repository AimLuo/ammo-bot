# 05: spec 落成 EPIC 和落成 PLAN 时的语义差异

Type: grilling
Status: resolved
Blocked by: 01
Labels: wayfinder:grilling

## Question

to-spec 询问“生成 EPIC 还是 PLAN”后，两种选择在 markplane 中各意味着什么：spec 正文放在哪、to-tickets 产出的实现工单如何挂接（TASK 的 `epic` 字段，还是 `plan` 字段 / PLAN 的 `implements`）、implement-spec 如何据此找到任务图、完成时 spec 条目如何关闭；`ready-for-agent` 等 triage 标签映射到 markplane 的 tags 还是 status workflow？

## Answer

2026-10-09 与用户 grilling 确认（全部采纳推荐）：

1. **选择依据**（my-to-spec 每次都问，并按对话给出推荐）：EPIC = 面向用户的功能/成果，上 roadmap、工单多、周期长；PLAN = 偏技术的方案（重构、迁移、内部机制），范围小，可从属于已有 EPIC。
2. **正文模板**：两者都用 to-spec 模板（Problem Statement / Solution / User Stories / Implementation Decisions / Testing Decisions / Out of Scope / Further Notes）；markplane 新增 `spec` 模板，在 manifest 中同时注册到 epic 与 plan。PLAN 不再叠加 Phases / Rollback 等节。
3. **实现工单挂接**：EPIC → `Task.epic`；PLAN → `Task.plan`（`Plan.implements` 双向自动维护，tracker 文档注明这是关系簿记，不算违反 to-tickets 的“不修改父 issue”）。CLI / MCP 需支持创建不挂 TASK 的独立 PLAN。
4. **triage 标签**：给 PLAN 增加 `tags` 字段；所有 triage 角色统一映射为同名 tag（`ready-for-agent` 等），spec 与工单一致。
5. **spec 状态生命周期**（即 tracker 文档的“how this tracker closes work”）：

   | 时机 | EPIC | PLAN |
   |------|------|------|
   | my-to-spec 发布 | `next` | `approved` |
   | implement-spec 开始 | `now` | `in-progress` |
   | implement-spec 完成 | `done` | `done` |

6. **找任务图**：EPIC 用 `ls --epic`；PLAN 新增 `ls --plan` 过滤，MCP query 同步支持（具体命令归“技能和 markplane 的交互接口”）。
7. **map → spec 回链**：对话来自某张 map 时，my-to-spec 发布后自动 `link <MAP> -r spec`；但不改 map 状态——map 的 `done` 由 wayfinder 在无剩余工单时设置（需在 tracker 文档的 Wayfinding operations 中写明，归“技能和 markplane 的交互接口”）。
8. **术语**：`GLOSSARY.md` 新增 Implementation ticket。
