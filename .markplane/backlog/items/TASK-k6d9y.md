---
id: TASK-k6d9y
title: Run 生命周期与任务状态映射
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-8aavc
depends_on: []
blocks:
- TASK-2vpdd
- TASK-pd8bj
related: []
assignee: chuck
tags: []
position: ag
created: 2026-10-10
updated: 2026-10-10
---

# Run 生命周期与任务状态映射

## Question

一个 Run 从被扫描到结束，Implementation ticket 的状态、标签、assignee 如何变化？具体：

- 被拾取时改什么（如 `planned` → `in-progress`、assignee 设为谁）以防被再次扫描；merge 成功后是否 `done`。
- 什么算失败：非零退出、verdict 行缺失、合并冲突解决不了、是否有每 Stage 超时；失败时除加 `ready-for-human` 外，状态与 `ready-for-agent` 标签如何处理。
- worktree 与分支命名（如 `.worktrees/<TASK-id>`、分支名），成功 / 失败后各自是否删除。
- 名额满时的拾取顺序（priority / position / 创建时间）。

## Answer

### 事实来源与写入权

- 主工作区的 `.markplane/` 是运行期间 ticket 状态的唯一事实来源。Dispatcher 给 agent 子进程设环境变量（如 `MARKPLANE_ROOT=<主工作区>`），CLI 遵守它，agent 的所有 `markplane` 调用都落到主工作区；worktree 里的 `.markplane/` 只读，分支不提交 `.markplane/` 改动。（CLI 目前没有指定 root 的方式，需要新增。）
- 只有 Dispatcher 改状态、标签、assignee；agent 只追加评论，prompt 明确禁止 agent 执行 `markplane done` / `status` / `update`。
- 所有判断按 workflow 状态**类别**（planned / active / completed），不写死状态名。

### 状态映射

| 时机 | 状态 | 标签 | assignee | 其他 |
| --- | --- | --- | --- | --- |
| 拾取 | active 类别首个（`in-progress`） | 不变 | `dispatcher` | 占一个全局名额直到 Run 结束（含排队等 merge） |
| 成功（merge 完成） | completed 类别首个（`done`） | 保留 `ready-for-agent` | 保留 `dispatcher` | 评论：目标分支、合并提交 SHA、是否已推送；只提交该 ticket 文件（按路径提交，带 `Refs: TASK-xxx`），再按开关推送 |
| 失败 | 保持 `in-progress` | 移除 `ready-for-agent`，加 `ready-for-human` | 保留 `dispatcher` | 评论：失败的 Stage、原因、会话 ID；立即释放名额 |
| serve 关闭中断 | 保持原状 | 不变 | 不变 | 不算失败，交给重启恢复 |
| 人在运行中改了 ticket | — | — | — | 每个 Stage 开跑前（含 merge 排队结束）重读；状态不再是 active 或 assignee 不再是 `dispatcher` 时静默停止：不再写 ticket、释放名额、保留 worktree；不中途杀进程 |

中间状态（in-progress、失败标签、评论）只写进主工作区文件、不提交。因此拾取状态不跨机器共享：默认同一仓库只有一个 Dispatcher。

### 失败判定

任一情况即该 Stage 失败、Run 转人工：

1. agent 进程非零退出；
2. implement 末行没有合法的 `REVIEW: clean|issues`；
3. implement 结束后分支相对基点没有新提交；
4. fix 末行不是 `FIX: done`（fix 以 `FIX: done|blocked` 收尾）；
5. merge 无法解决冲突，或合并后主工作区分支不包含该分支；主工作区未提交改动挡住合并也算失败（不 stash、不碰用户改动）；
6. 单个 Stage 超时（默认 60 分钟，可配置，配置位置见 [[TASK-cq846]]），被杀算失败。

流程：`REVIEW: clean` → merge；`REVIEW: issues` → 一轮 fix → merge，不再 review 循环。

### worktree 与分支

- 路径 `.worktrees/<TASK-id>`，分支 `agent/<TASK-id>`；基点 = 拾取时主工作区当前分支 HEAD；merge 目标 = 合并时主工作区当前检出的分支。
- 成功后删除 worktree 和分支；失败时两者保留，供 comment and continue 使用。
- 人把失败 ticket 改回 `planned` 并重新加 `ready-for-agent` 时，再次拾取会删掉旧 worktree 和分支，从头开始。

### 拾取顺序

跨 Project 轮流取（每轮每个 Project 一个）；同一 Project 内按 priority（critical → someday）→ position → created → id 排序。
