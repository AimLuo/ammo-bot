# 13: 新功能的测试策略

Type: grilling
Status: resolved
Blocked by: None
Labels: wayfinder:grilling

## Question

MAP、决策工单（`map` 字段、类型约束、互斥校验）、frontier 查询、`resolve` / `comment` / 独立 PLAN 等新功能在哪些 seam 上测试（core 单元测试、CLI 集成测试、MCP 集成测试、Web UI）？以上游现有测试（`crates/markplane-cli/tests/integration.rs`、`mcp_integration.rs`、core 内 tests）为先例，哪些必须覆盖、哪些不测？

## Answer

2026-10-09 与用户 grilling 确认（全部采纳推荐）：

1. **主 seam = CLI 集成测试**（`assert_cmd` + 临时目录 `init --empty`，先例 `crates/markplane-cli/tests/integration.rs`）：只测外部行为（命令输出、生成文件的 frontmatter 与正文），实现时按 tdd 先写。必须覆盖：`markplane map` 带正文创建；`add --map` 类型校验及与 `epic` / `plan` 互斥；`ls --frontier` 过滤与 `position` 排序（含 cancelled blocker 视为解除）；`resolve` 的三个效果（`## Answer`、done、map 追加决策行）；`comment`；独立 `plan` 与 `--plan`；`link -r map` / `-r spec`；归档 map 连带工单；`Plan.tags`。
2. **MCP 集成测试**（先例 `mcp_integration.rs`）：每个新增 / 修改的 tool 与 resource 一组对齐测试，验证与 CLI 行为一致，不重复 CLI 已覆盖的细节——`markplane_add`（正文、map、plan）、`markplane_query`（新过滤与返回字段）、`markplane_comment`、`markplane_resolve`、`markplane://maps`、`markplane://map/{id}`。
3. **core 单元测试**（先例 `project.rs`、`context.rs` 内 tests）：只用于纯逻辑——frontier 计算、MAP 正文按标题切节、`find_blocked_items` 的 cancelled 处理、`.context` 生成（Active Maps、`maps.md`、决策工单不进 Priority Queue / metrics / blocked）。
4. **Web UI**：不引入前端测试框架；`tsc` + lint + build 兜底，用迁入的本 map 真实数据人工检查 `/maps` 列表页与路线图页。
5. **my-* 技能**：不写自动化测试，以自举为验收并写进对应实现工单——本仓库运行 my-setup 后 `docs/agents/issue-tracker.md` 可用；本 map 迁入后用 wayfinder 在 markplane 上完成一次 claim + resolve；用 my-to-spec 发布一份小 spec 并确认会询问 EPIC 还是 PLAN。
