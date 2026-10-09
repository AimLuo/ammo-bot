---
id: TASK-3mkf9
title: 核对各技能对 tracker 的依赖点
status: done
priority: medium
type: research
effort: medium
epic: null
plan: null
map: MAP-vs3wh
depends_on: []
blocks:
- TASK-hebkk
- TASK-fv9bd
- TASK-2qsvh
- TASK-j72uw
related: []
assignee: null
tags: []
position: aG
created: 2026-10-09
updated: 2026-10-09
---

# 核对各技能对 tracker 的依赖点

## Question

逐个阅读 `.tmp/skills` 中的 wayfinder、grill-with-docs（及其调用的 grilling / domain-modeling）、to-spec、to-tickets、implement、implement-spec、code-review、setup-matt-pocock-skills（含 `issue-tracker-*.md`、`triage-labels.md`、`domain.md`）：

1. 每个技能对 issue tracker 做了哪些操作（发布、读取、子 issue、阻塞、claim、关闭、标签、评论、PR 关联等），各自以什么措辞依赖 tracker 文档？
2. 汇总成“markplane tracker 文档必须表达的操作清单”。
3. 哪些技能只靠新增一份 `issue-tracker-markplane.md` 就能工作，哪些必须改写成 `my-*` 版本（例如 to-spec 需要询问 EPIC 还是 PLAN），原因是什么？
4. markplane 现有能力（`.tmp/markplane` 的 CLI 命令、MCP 工具、Task/Epic/Plan/Note 字段）能覆盖清单中的哪些操作，缺口在哪？

## Answer

详情：分支 `research/skill-tracker-deps`（worktree `.worktrees/research-skill-tracker-deps`）的 `research/skill-tracker-deps.md`，commit `1517841`。

1. **各技能的 tracker 操作**
   - wayfinder 最重：发布 map、建子工单、原生阻塞、frontier 查询、claim、resolution 评论、关闭、编辑 map body、删单。措辞是“Consult the tracker doc's "Wayfinding operations" section”。
   - to-spec：发布 + `ready-for-agent`。
   - to-tickets：读 body 和评论、按依赖序发布、原生阻塞、sub-issue（“tracker doc's operation”）、`ready-for-agent`。
   - implement：只 fetch。
   - implement-spec：读任务图、“the way the issue tracker closes work”、可选的 PR 关联。
   - code-review：按 commit message 里的 issue 引用 fetch（“via the workflow in the tracker doc”）。
   - grill-with-docs / grilling / domain-modeling：零 tracker 操作。
2. **markplane tracker 文档必须表达的操作（22 条）**
   - 基础：建条目带 body、读、按 parent/状态/标签/assignee 查询、挂 parent（EPIC/PLAN/MAP）、评论、标签、done/cancelled、编辑 body、删除。
   - 映射：publish/fetch 映射；新增“how this tracker closes work”（不走 PR）和 commit 引用格式。
   - Wayfinding：Map / Child / Blocking / 有序 Frontier / Claim / Resolve / 按名称引用。
   - spec 扩展：spec→工单的列举，以及 EPIC/PLAN 的状态约定。
3. **哪些需要 fork**
   - 原样可用：grill-with-docs、grilling、domain-modeling、implement、implement-spec、code-review、to-tickets、wayfinder。它们都通过可被 tracker 文档改写的抽象措辞依赖 tracker；local tracker 已经证明 label/assign 可以换形态。
   - 必须 fork：
     - `my-to-spec`：要问 EPIC 还是 PLAN，而上游写了“Do NOT interview”；PLAN 没有 tags，需要分支说明。
     - `my-setup-matt-pocock-skills`：tracker 选项和模板写死在技能里，这是分发 markplane 模板的唯一入口。
4. **覆盖情况**
   - 已覆盖：原生阻塞（双向维护 + 环检测）、TASK→EPIC/PLAN 链接、claim（`start`/assignee）、done/cancelled、Task/Epic/Note 的 tags、`show` 读取、按 epic 查询。
   - 缺口：
     - ① MAP 类型 + `Task.map` 整套没有。
     - ② 没有 frontier 组合查询；随机 ID 没有顺序；cancelled 的 blocker 仍算阻塞。
     - ③ 没有评论模型或 body 追加。
     - ④ CLI/MCP 创建时不收 body，MCP 也不能改 body。
     - ⑤ CLI/MCP 创建 PLAN 必须挂 TASK；PLAN 没有 tags。
     - ⑥ 不能按 plan/map 或“未分配”查询；MCP query 返回字段太少。
