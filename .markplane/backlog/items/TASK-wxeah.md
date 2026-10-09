---
id: TASK-wxeah
title: 仓库自身的 .markplane 数据如何处理
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
position: aO
created: 2026-10-09
updated: 2026-10-09
---

# 仓库自身的 .markplane 数据如何处理

## Question

上游 markplane 自身的 `.markplane/`（其 backlog / roadmap / plans / notes，26 个开放条目）是保留、清空还是替换为本项目的数据？本 `.scratch/markplane-skills` map 及其工单在 MAP 类型实现后是否、何时、以何种方式迁入 `.markplane`（作为 MAP + 决策工单），`.scratch` 是否随之删除？

## Answer

2026-10-09 与用户 grilling 确认（全部采纳推荐）：

1. **上游条目数据**：删除 `.markplane/` 下 backlog / roadmap / plans / notes 的全部条目与归档（含 notes 下的 `decisions.md`、`ideas.md`），保留 `config.yaml` 与 `templates/`（模板是产品的一部分，后续加入 `spec`、`map` 模板）。将来合并上游若出现“修改 vs 删除”冲突，一律取删除（细节归“如何持续同步上游 markplane 的更新”）。
2. **`config.yaml`**：`project.name` 保留 “Markplane”，description 改为说明本 fork 是 mattpocock skills 的 tracker 后端；`task_types` 不加 prototype / grilling（决策工单类型内置）。
3. **本 map 的最终 spec**：用上游现有 CLI 发布为 `.markplane` 中的 EPIC——`markplane epic` 创建后直接编辑文件写入正文；工单用 `markplane add --epic` + `link -r blocks` + `ready-for-agent` 标签。不走 `.scratch/.../spec.md`。
4. **本 map 迁入 `.markplane`**：作为该 spec 中的一张实现工单——MAP 功能落地后，把本 map 转为 MAP 条目、决策工单转为带 `map` 的 TASK，保留阻塞关系与 `## Answer`，并作为路线图页面的真实数据；迁移后 `link <MAP> -r spec` 指向上面的 EPIC，删除 `.scratch/markplane-skills/`，保留 `research/*`、`prototype/*` 分支供工单中的链接引用。
