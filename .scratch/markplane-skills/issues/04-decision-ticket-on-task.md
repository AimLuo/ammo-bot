# 04: 决策工单在 TASK 上怎么表示

Type: grilling
Status: resolved
Blocked by: 01
Labels: wayfinder:grilling

## Question

wayfinder 的决策工单复用 TASK 时：类型取值 research / prototype / grilling / task 如何与 markplane 现有的 task `type` 共存；`map` 字段的语义；claim（assignee）与 resolve（状态 + `## Answer`）具体落在哪些字段、哪个状态；frontier（open、未阻塞、未 claim）如何计算；如何保证决策工单不被当成实现工单领取（不带 `ready-for-agent`）？

## Answer

2026-10-09 与用户 grilling 确认（全部采纳推荐）：

1. **类型字段**：复用 `Task.type`。挂了 `map` 的 TASK，其 `type` 必须是内置的 `research / prototype / grilling / task` 之一，并跳过 `config.task_types` 校验；同名 `research` 靠是否有 `map` 区分。
2. **`map` 字段**：单值 `Option<MAP-id>`，经 `link -r map` 设置，再 link 即改挂；有 `map` 的 TASK 即 Decision ticket；与 `epic` / `plan` 互斥，由 core 校验。
3. **状态语义**（按 StatusCategory 表述，不写死状态名）：新建进入 backlog 分类；**claim** 以 assignee 非空为准，执行方式为 `markplane start --user`（同时进入 active 分类）；**resolve** 进入 completed 分类（`markplane done`）；**判为范围外** 进入 cancelled 分类。
4. **解决结论**：写在决策工单正文末尾的 `## Answer` 一节；通用评论模型留给“技能和 markplane 的交互接口”。
5. **阻塞判定**：全局修改 `find_blocked_items`，blocker 处于 completed 或 cancelled 分类均视为已解除。
6. **frontier 顺序**：用现有 `position` 分数索引（创建先后自动生成，可拖拽调整）；决策工单统一默认 priority。frontier = 某 map 下 open、所有 `depends_on` 已关闭、assignee 为空的 TASK，按 `position` 排序。
7. **与实现工单的界线**：决策工单永不带 `ready-for-agent`（tracker 文档约定）；Backlog / 看板默认隐藏带 `map` 的 TASK，提供开关显示，在 MAP 页面展示。
8. **research 产出**：保持上游做法——`research/<name>` 分支 + 工单内 context pointer，不改用 NOTE（避免 fork wayfinder）。
9. **术语**：`GLOSSARY.md` 新增 Decision ticket、Frontier。
