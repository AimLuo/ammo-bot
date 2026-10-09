# 06: 技能和 markplane 的交互接口

Type: grilling
Status: resolved
Blocked by: 03, 04, 05
Labels: wayfinder:grilling

## Question

技能读写 markplane 时以 CLI 为主还是 MCP 为主（或两者都写进 tracker 文档）？为支撑 01 列出的操作清单与 03/04/05 的模型，需要新增或修改哪些 CLI 命令与 MCP 工具（例如 map 创建、frontier 查询、claim、resolve、追加评论）？

另需覆盖：带正文创建条目、独立 PLAN 创建、`ls --plan` / MCP 按 plan 与 map 过滤、通用评论模型（to-tickets 会读评论）、`link -r map` / `link -r spec`，以及 tracker 文档 Wayfinding operations 中“map 何时、由谁设为 `done`”。

## Answer

2026-10-09 与用户 grilling 确认（全部采纳推荐）：

1. **以 CLI 为准**：tracker 文档只写 CLI 路径；MCP 补齐同等能力但不写进 tracker 文档。
2. **正文写入**：创建命令加 `--body-file <path|->` 一步写入完整正文；局部修改（如 map 的某节）继续直接编辑文件，不做按节命令。
3. **通用评论**：所有条目正文末尾 `## Comments`，每条为 `### <日期> <作者>` 小节；新增 `markplane comment <ID> --body-file f`。决策工单结论仍写 `## Answer`。
4. **Wayfinding 操作**：
   - 创建 map：`markplane map "<title>" --body-file f`
   - 决策工单：`markplane add "<title>" --map MAP-x --type <research|prototype|grilling|task>`
   - 阻塞：`markplane link A B -r blocks`
   - frontier：`markplane ls --map MAP-x --frontier`；`--frontier` 为通用过滤（open、所有 `depends_on` 已关闭、assignee 为空，按 `position` 排序），可与 `--epic` / `--plan` 组合
   - claim：`markplane start ID --user <name>`
   - resolve：新命令 `markplane resolve ID --answer-file f --gist "<一句话>"`，原子地写 `## Answer`、置 done、在所属 map 的 Decisions so far 追加 `- 标题 [[ID]]: gist`
   - 范围外：`markplane status ID cancelled` + 直接编辑 map 的 Out of scope
   - map done：`resolve` 后若 map 无 open 工单则打印提示；wayfinder 询问用户确认后执行 `markplane status MAP-x done`（写入 tracker 文档 Wayfinding operations）
5. **Spec 操作**：
   - EPIC：`markplane epic "<t>" --body-file f --tags ready-for-agent`
   - PLAN：`markplane plan [TASK_ID] [--title <t>] [--body-file f] [--tags …]`（带 TASK_ID 时行为同上游，否则独立 PLAN）
   - 实现工单：`markplane add "<t>" --epic X` / `--plan X`（新增 `--plan`）
   - map 回链：`markplane link MAP-x EPIC-y -r spec`
   - 找工单：`markplane ls --epic X` / `--plan X`
6. **MCP 对齐**：`markplane_add` 支持正文及 map / plan 类型；`markplane_query` 支持 map / plan / frontier 过滤，返回补 assignee / depends_on / tags；新增 `markplane_comment`、`markplane_resolve`。
7. **commit 引用**：commit message 末尾 trailer `Refs: TASK-xxxxx`，code-review 用 `markplane show` 读取。
