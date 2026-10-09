---
id: MAP-vs3wh
title: 以 markplane 为底适配 mattpocock skills
status: done
specs:
- EPIC-7m4ss
related: []
tags: []
created: 2026-10-09
updated: 2026-10-09
---

# 以 markplane 为底适配 mattpocock skills

## Destination

一份完整 spec，能直接交给 `to-tickets` 拆单：本仓库成为 markplane 的 fork（结构、命名与 markplane 一致），并作为 mattpocock skills（wayfinder / grill-with-docs / to-spec / to-tickets / implement / implement-spec / code-review）的 issue tracker 后端；新增 MAP 条目类型与页面；需改造的技能以 `my-*` 形式放在仓库根目录。

## Notes

- 源素材：`.tmp/markplane`（产品底座）、`.tmp/skills`（mattpocock skills 源码，tracker 相关见 `skills/engineering/setup-matt-pocock-skills/issue-tracker-*.md`）。
- 默认只规划不实现；实现阶段另用 `implement-spec` 与 `code-review`。
- 每个 grilling 工单调用 grilling + domain-modeling。
- 语言：代码、技能、UI 用英文；map、工单、讨论用中文。
- 调研原始材料：分支 `research/skill-tracker-deps`；页面原型：分支 `prototype/map-page`（选定 Variant C）。
- 本 map 已从 `.scratch/markplane-skills/` 迁入 `.markplane`（TASK-9hqi2）。

已定的起点决策（charting 时确认）：

- markplane 源码整体拷入仓库根目录，保留 `markplane-*` crate 名与 `markplane` 命令。
- markplane 既做技能的 tracker 后端，仓库同时携带 `my-*` 技能；未改动的上游技能直接用上游，不拷贝。
- 概念映射初稿：spec → EPIC 或 PLAN（to-spec 时询问）；实现工单 → TASK（`depends_on`/`blocks` 表达阻塞）；wayfinder map → 新条目类型 MAP + 新页面；决策工单 → 复用 TASK，加 `map` 字段，类型沿用 research/prototype/grilling/task。
- 旧 ammo-bot 内容全部丢弃，仅保留 git 历史。

## Decisions so far

- 核对各技能对 tracker 的依赖点 [[TASK-3mkf9]]: 只需 fork my-to-spec 和 my-setup-matt-pocock-skills，其余技能靠 issue-tracker-markplane.md 原样可用；markplane 主要缺 MAP/map 字段、frontier 查询、评论/body 写入、独立 PLAN 与 PLAN 标签

- 把 markplane 拷入仓库根目录并跑通构建和测试 [[TASK-raz23]]: 已从上游 73cce05 导出到根目录，build / test / clippy / Web UI / embed-ui 全部通过；上游 .markplane/ 数据随之拷入

- MAP 条目的数据模型 [[TASK-hebkk]]: MAP-xxxxx 存 maps/，状态 active/done/abandoned，Destination 只在正文，MAP 单向持有 specs，工单侧 Task.map 单值，Decisions 条目写 标题 [[ID]]: gist，归档 map 连带子工单

- 决策工单在 TASK 上怎么表示 [[TASK-fv9bd]]: 复用 type（挂 map 时限内置四值），map 单值且与 epic/plan 互斥，claim=assignee（start --user）、resolve=completed、范围外=cancelled，结论写 ## Answer，cancelled blocker 全局视为解除，frontier 按 position 排序，看板默认隐藏决策工单

- spec 落成 EPIC 和落成 PLAN 时的语义差异 [[TASK-2qsvh]]: my-to-spec 每次询问并推荐（EPIC=用户功能、PLAN=技术方案），两者共用 to-spec 正文模板，工单经 epic/plan 挂接，PLAN 补 tags 且可独立创建，triage 角色统一为同名 tag，状态 EPIC next→now→done / PLAN approved→in-progress→done，my-to-spec 自动回链 map 但不改 map 状态

- 技能和 markplane 的交互接口 [[TASK-9yxpq]]: tracker 文档只写 CLI，MCP 对齐；创建加 --body-file，局部改动直接编辑文件；新增 map / comment / resolve 命令、add --map/--plan、ls --map/--plan/--frontier、独立 plan；commit 用 Refs: trailer

- MAP 页面原型 [[TASK-w5rd5]]: 详情页用路线图（依赖层级 DAG，Start→Destination，点节点看 Question/Answer），另设 /maps 列表页，工单详情可跳回 map 并高亮节点；原型存于 prototype/map-page 分支

- my-* 技能清单和分发安装方式 [[TASK-j72uw]]: 只做 my-to-spec 与以上游为底的 my-setup-matt-pocock-skills（加 Markplane 选项、替换本机 Beads 版）；仓库提供 scripts/link-skills.sh，本人用 skills-manager 导入；上游技能不分发

- 仓库自身的 .markplane 数据如何处理 [[TASK-wxeah]]: 删除上游全部条目与归档，保留 config 与 templates；最终 spec 用现有 CLI 发成 .markplane 里的 EPIC；MAP 功能落地后由一张实现工单把本 map 迁入并回链 EPIC，再删 .scratch

- .context 摘要与 MCP resources 如何包含 map [[TASK-dugug]]: summary 加 Active Maps（≤3 张，含进度与 next），新增 .context/maps.md 与 maps/INDEX.md；决策工单不进 Priority Queue / metrics / blocked，只在 active-work 出现；MCP 加 markplane://maps 与 markplane://map/{id}

- 如何持续同步上游 markplane 的更新 [[TASK-sad8p]]: 加 upstream remote 并以 -s ours 空合并 73cce05 建立祖先，之后按需三方合并；版本号用 0.1.4+skills.N；发布渠道文件原样保留，只支持源码安装

- README、docs 与 CLAUDE.md 的改写范围 [[TASK-vk4yg]]: README 加 fork 说明并只留源码安装；原地更新 6 份参考文档并新增 docs/skills-workflow.md；个人规则并入 CLAUDE.md、AGENTS.md 改软链；claude-md 片段加 maps；本仓库自跑 my-setup 作为实现工单

- 新功能的测试策略 [[TASK-8p5hb]]: CLI 集成测试为主 seam（tdd，只测外部行为），MCP 做对齐测试，core 单测只覆盖 frontier / 切节 / blocked / context 生成；UI 不加测试框架，人工核对；my-* 技能以自举为验收

## Not yet specified

<!-- 已全部转为工单 09–13 -->

## Out of scope

- 产品改名（继续叫 markplane）。
- 旧 ammo-bot 的 scanner / pipeline / PM2 / bun 等全部内容。
- 与 tracker 无关的 mattpocock 技能（tdd、teach 等）的改造。
