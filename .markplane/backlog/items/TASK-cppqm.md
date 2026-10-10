---
id: TASK-cppqm
title: Dispatcher 主干：拾取 → implement → merge → done
status: planned
priority: medium
type: feature
effort: large
epic: EPIC-ce5ps
plan: null
depends_on:
- TASK-hs89w
- TASK-gbq58
- TASK-v4j2t
- TASK-hkj4u
- TASK-72tr5
blocks:
- TASK-in6ej
- TASK-mzkqb
related: []
assignee: null
tags:
- ready-for-agent
position: b00
created: 2026-10-10
updated: 2026-10-10
---

# Dispatcher 主干：拾取 → implement → merge → done

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

Dispatcher 主干：打开 Scan switch 后，`ready-for-agent` 的 Implementation ticket 被自动拾取，在独立 worktree 里跑 implement，`REVIEW: clean` 后合并回主工作区并标为 done。本票只做全程顺利的路径（单 Project 内，遵守并发上限）；任何不顺利的结局本票只在 Run record 记 `failed` 和原因，转人工在后续票。Dispatcher 代码放独立模块，不继续堆进现有 serve 文件。

- 扫描与拾取：已打开 Scan switch 的 Project 每 15 秒扫描（测试用显式 tick 钩子）。候选为状态在 planned 类别、带 `ready-for-agent`、未被阻塞（复用现有"被阻塞"判定）的 Implementation ticket；Project 内按 priority → position → created → ID。拾取写入：状态改为 active 类别首个状态、assignee 改为 `dispatcher`。Run 从拾取到结束占一个 Slot，不超过并发上限。候选筛选与排序为 core 纯函数并有单元测试。
- Run：建 worktree 前 `git worktree prune`；worktree `.worktrees/<TASK-id>`、分支 `agent/<TASK-id>`，基点为拾取时主工作区当前分支 HEAD。所有状态转换先写 Run record（格式由评论 Spec 定义）再执行；每次写 Run record 后推 `run_changed`（Project 通道，负载 task_id、序号、显示状态、Stage）。
- agent 调用契约（见父 Spec「agent 调用契约」）：固定参数、模型与超时取 Stage 开跑时的快照、cwd 为工作目录、stdin 接 `/dev/null`、单独进程组、继承 serve 环境并加 `MARKPLANE_ROOT=<主工作区>` 与 `MARKPLANE_AUTHOR=agent-<stage>`；`system/init` 的 `session_id` 立即写入 Run record；记录 `usage`、`duration_api_ms`、stderr 末尾。stream-json 解析与末行判定（最后一个 `assistant` 事件文本的最后一个非空行；成功 = 退出码 0 且有 `result/success`）为 core 纯函数并有单元测试。
- prompt：按原型分支 `prototype/stage-prompts` 实现公共片段（无人值守、worktree 边界、ticket 规则、末行约定）与 implement（`/implement <TASK-id>`，`**Assumptions**` / `**Review**` 评论，`REVIEW: clean|issues`）、merge（`--no-ff --no-commit`、验证后提交、`MERGE: done|failed`）模板。
- 合并：本 serve 内每个工作树（per-worktree git dir 的 realpath）一条 FIFO Merge queue，Run 进入时为 `merge-queued`，仍占 Slot；merge 前重读 ticket；合并后校验目标分支包含该分支。
- 成功收尾（幂等）：ticket 改为 completed 类别首个状态、保留标签与 assignee；只按路径提交该 ticket 文件并带 `Refs: TASK-xxx`；按 `push_after_merge` 推送；`dispatcher` 署名评论写目标分支、合并 SHA、是否已推送。中间状态只写主工作区文件不提交。
- 测试：可替换的假 agent 脚本（按场景输出 stream-json、在工作目录提交、写评论、打印末行），每个测试一个真实临时 git 仓库加 Project，通过 API 驱动并检查 ticket 文件、Run record、git 状态和事件。

## Acceptance criteria

- [ ] 只拾取 planned、带 `ready-for-agent`、未被阻塞的 Implementation ticket，顺序符合规则；拾取后状态为 active、assignee 为 `dispatcher`；同一 ticket 不会被拾取两次
- [ ] 同时运行的 Run 不超过并发上限
- [ ] happy path：假 agent `REVIEW: clean` → merge → ticket 为 done，主分支包含合并提交，Dispatcher 只提交了该 ticket 文件且带 `Refs:` 尾注，成功评论内容正确
- [ ] `push_after_merge` 开 / 关时分别推送 / 不推送（用本地裸仓作远端）
- [ ] agent 进程收到 `MARKPLANE_ROOT` / `MARKPLANE_AUTHOR`，其评论落在主工作区 ticket 文件
- [ ] Run record 记录 session_id、模型与超时快照、usage；每次写入都推 `run_changed`
- [ ] 同一工作树的两个 Run 合并串行
- [ ] 重复执行成功收尾不产生多余提交或推送
- [ ] core 单元测试覆盖候选筛选排序与 stream-json 末行判定
