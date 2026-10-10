---
id: TASK-6hqkh
title: 遗留 worktree 与分支的清理
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-8aavc
depends_on:
- TASK-pd8bj
blocks: []
related: []
assignee: chuck
tags: []
position: an
created: 2026-10-10
updated: 2026-10-10
---

# 遗留 worktree 与分支的清理

## Question

失败或静默停止的 Run 会保留 `.worktrees/<TASK-id>` 和 `agent/<TASK-id>` 分支（见"Run 生命周期与任务状态映射"[[TASK-k6d9y]]）。这些遗留物何时、由谁清理？

- 人把 ticket 直接改成 `done` / `cancelled`（不再走 comment and continue）时，Dispatcher 是否自动删除；分支上有未合并提交时怎么办。
- serve 崩溃或重启后，磁盘上存在但已无对应活动 Run 的 worktree 怎么识别和处理（与重启恢复的答案相关）。
- 是否需要界面上的手动"清理"入口，还是只靠自动规则。

## Answer

## Answer

### 结论

清理靠 **Dispatcher 持锁时的对账**，不挂在归档、改状态等具体动作上。ticket 进入 completed / cancelled 类别后，Dispatcher 自动删掉 Run 留下的 worktree 和**已合并**的 `agent/<TASK-id>` 分支；有未提交改动的 worktree、有未合并提交的分支、孤儿 agent 还活着的情况一律跳过，不强删。详情面板 Run 段的操作位提供手动 `Clean up…`，确认后强制清理。Dispatcher 只碰 Run 记录里登记过的 worktree 和分支。Run 记录永远保留，不随归档删除。

### 已核实的代码事实

| 事实 | 位置 | 对本题的意义 |
| --- | --- | --- |
| 没有硬删除 ticket：Web 的 `DELETE /api/tasks/{id}` 和 MCP 都调用 `archive_item`（把文件移到 `archive/`），可以 `unarchive` 撤销 | `serve.rs` `delete_task`、`mcp/tools.rs`、`project.rs` `archive_item` | "随删除清理"等于"随归档清理"；归档可逆，所以 Run 记录不能跟着删 |
| 改 ticket 状态的途径有 CLI、MCP、Web、手改文件、`git pull`；没有统一的状态变更钩子；文件监听只推 `.md` / `config.yaml` 的薄事件 | `serve.rs` `run_file_watcher` | 挂钩子挡不全；按状态对账才可靠 |
| 状态类别里 completed 和 cancelled 是两个类别，`is_closed()` 同时覆盖二者 | `models.rs` `StatusCategory` | 自动清理条件用 `is_closed()` 判断，不写死状态名 |
| 本仓 `.worktrees/` 下已有 4 个人工建的 worktree（`prototype-*`、`research-*`），分支是 `prototype/*`、`research/*` | `git worktree list` | Dispatcher 不能按目录或分支前缀扫，只能按 Run 记录认领 |
| 代码里还没有任何 git 调用 | 全仓 grep | 清理用的 git 命令都要新写 |
| git 行为：`git worktree remove` 遇到已跟踪文件改动**或**未跟踪文件时拒绝，需要 `--force`；目录已经不在时它会报错，需要 `git worktree prune`；`git branch -d` 只删已合并进当前 HEAD 的分支 | git 2.54 | 自动清理不加 `--force` 就天然安全；目录丢失要靠 prune 收尾 |

### 1. 清理的对象与认领范围

- 只清理 **Run 记录里登记的** worktree 路径（按 realpath 和 `git worktree list` 比对）和分支 `agent/<TASK-id>`。没有 Run 记录的 worktree 或 `agent/*` 分支一律不碰，包括人工建的、别的机器或别的 Project 建的（同一仓库的多个 Project 共用分支命名空间，见 [[TASK-a8246]]）。
- 只看每个 ticket **最新一次** Run 的遗留物；更早的 Run 在重新拾取时已经删过（[[TASK-k6d9y]]）。
- 所有 Dispatcher 发起的清理（自动或手动）都要先持有该 Project 的 Dispatcher 锁（[[TASK-2nt8u]]）。清理不经过合并锁：可清理的 Run 都已是终态，不会和 merge 同时操作同一个分支。

### 2. 能不能清理（硬性前提）

| Run 显示状态 | ticket 类别 | 自动清理 | 手动 `Clean up…` |
| --- | --- | --- | --- |
| `running` / `merge-queued` / stopping / `running elsewhere` | 任意 | 否 | 否 |
| `interrupted` | 任意 | 否 | 否（要先恢复，或改 ticket 让对账写成 `stopped`） |
| `failed` | active | 否 | 否（continue 还要用） |
| `failed` / `stopped` | completed / cancelled，或 ticket 文件已不存在 | **是** | 是 |
| `failed` / `stopped` | planned / backlog / draft | 否（人可能还要看；重新拾取时按 [[TASK-k6d9y]] 删） | 是 |
| `stopped` | active（人接手了） | 否 | 是 |
| `succeeded`，但成功收尾时没删掉 | 任意 | 每轮对账重试 | 是 |

- 归档不改变判断：只看 ticket 的状态类别（归档的文件照样能读）。归档时仍是 `in-progress` 的 ticket 不会被自动清理。
- 孤儿 agent 还活着（按 [[TASK-pd8bj]] 的 PID 加命令行探测）时，一律不自动删 worktree；每次动手前都重新探测一次。

### 3. 自动清理

- **时机**：(a) 取锁时的对账（[[TASK-pd8bj]] 已有）；(b) 持锁期间每次扫描（15 秒）；(c) Run 成功收尾时（[[TASK-k6d9y]] 已有）。不持锁时（扫描关、没有活动 Run）不清理，遗留物留到下次取锁再处理，期间 Run 段照常显示。
- **worktree**：用 `git worktree remove`，**不加 `--force`**。有任何已跟踪文件改动或未跟踪文件时 git 会拒绝：Dispatcher 跳过，记下原因 "uncommitted changes"，保留 worktree，等手动清理。
- **分支**：在 Project 所在工作树里执行 `git branch -d agent/<TASK-id>`，即只删已合并进该工作树当前 HEAD 的分支。有未合并提交（含人直接改成 `done` 但没合并的情况）就保留分支，记下 "unmerged commits (n)"。worktree 和分支各自判断：worktree 删掉、分支保留是正常结果，提交都还在分支上。
- **目录已丢失**：Run 记录里的 worktree 目录不在了，但 git 里还登记着 → 执行 `git worktree prune`。它只会清掉目录已经不存在的 worktree 登记，跟 git gc 自己做的一样，而且会跳过 locked 的 worktree。重新拾取前建 worktree 时同样先 prune，避免 `worktree add` 因为旧登记失败。
- **目录在但 git 不认**（不是一个已登记的 worktree）：不删目录，记原因，只能手动处理。
- 同一种跳过原因只在 `serve.log` 记一次（状态变化时才记），不会每 15 秒刷一次。

### 4. 手动 `Clean up…`

- **入口**：详情面板 Run 段头部的操作位（[[TASK-exz6h]] 预留的位置）。按第 2 节的表允许、且 worktree 或分支至少还剩一个时才出现。锁被别处持有时禁用，悬停提示持有者。
- **确认框**列出将要删除的内容：worktree 路径及未提交文件数；分支及未合并提交数（同时显示分支头 SHA）；如果孤儿进程还活着，提示 "will terminate pid N"。确认后**强制**清理：`git worktree remove --force`（locked 的也要强制）、`git branch -D`、必要时 prune。
- **API**：`POST /api/runs/{task_id}/cleanup`，请求体可带 `{ "terminate_orphan": true }`。顺序：取锁（没持锁就临时取，用完按 [[TASK-2nt8u]] 的规则释放；取不到返回 409 `dispatcher_locked`）→ 重读 ticket 和 Run 状态，按第 2 节判断（不满足返回 409 `not_cleanable`）→ 孤儿还活着但没带 `terminate_orphan` 返回 409 `orphan_running`，带了就先杀进程组（SIGTERM，5 秒后 SIGKILL）→ 清理 → 写 Run 记录 → 推 `run_changed`。
- 只做单个 Run 的清理，不做"全部清理"。

### 5. 重新拾取时的删除（补充 [[TASK-k6d9y]]）

- 人把 ticket 改回 `planned` 并加 `ready-for-agent`，这本身就是"从头来"的明确意图。所以重新拾取时**强制**删除旧 worktree 和分支，包括未提交改动和未合并提交。删除前把旧分支头 SHA 写进旧 Run 记录，之后仍可以用 reflog 或 `git branch <name> <sha>` 找回（直到 git gc）。
- 旧 Run 的孤儿 agent 还活着时，**本轮不拾取**这个 ticket（保持 `planned`），Run 段显示孤儿提示，等孤儿退出或人用手动清理杀掉它。

### 6. Run 记录

- **永远保留**，不随清理、归档或 unarchive 删除。文件很小，归档可逆，会话记录弹窗也要能看到已归档 ticket 的会话。
- 清理后在该 Run 上写：`worktree_removed_at`、`branch_removed_at`、`branch_head_at_removal`，以及最近一次跳过的原因 `cleanup_blocked`（`uncommitted_changes` / `unmerged_commits` / `orphan_running` / `not_a_worktree`，清理成功后清空）、`cleanup_trigger`（`auto` / `manual` / `repick` / `success`）。
- 不写 ticket 评论：清理是本机的事，ticket 上已有的评论（合并 SHA、失败原因）足够。

### 7. 界面

- **Run 段**头部下面加一行遗留物状态：`Worktree kept · 3 uncommitted files`、`Branch kept · 2 unmerged commits`、`Cleaned up <time>`。Details 折叠区里已删的 worktree 路径变灰，标 `(removed)`。
- 卡片的 `RunBadge` 不变（ticket 离开 active 后 failed / stopped 标记本来就不显示）；不做跨 Project 的遗留物清单。
- 自动或手动清理写完 Run 记录后，照 [[TASK-exz6h]] 推 `run_changed`，名额不受影响。

### 自问自答（均按推荐采纳）

**第 1 轮**

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 1 | 清理挂在归档 / 改状态的动作上，还是靠对账？ | 靠持锁时的对账；改状态的途径太多，挂钩子挡不全 |
| 2 | Dispatcher 能碰哪些 worktree 和分支？ | 只碰 Run 记录里登记的；人工建的、没有记录的一律不碰 |
| 3 | Run 记录要不要随归档或删除一起清理？ | 不要，永远保留；删除就是归档，而归档可逆 |

**第 2 轮**

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 4 | 人直接把 ticket 改成 `done` / `cancelled`，Dispatcher 自动清理吗？ | 清理；条件是 ticket 进入 completed / cancelled 类别，且 Run 是 `failed` / `stopped` |
| 5 | ticket 改回 planned 等类别时自动清理吗？ | 不清理；留给重新拾取或手动清理 |
| 6 | worktree 有未提交改动怎么办？ | 自动清理不加 `--force`，跳过并记原因 |
| 7 | 分支有未合并提交怎么办？ | worktree 照删，分支保留（`git branch -d`）；不自动强删 |
| 8 | 孤儿 agent 还活着？ | 不自动清理；每次动手前重新探测 |
| 9 | 要不要跑 `git worktree prune`？ | 只在本 Run 的 worktree 目录已经丢失、或重新拾取建 worktree 前跑；它只清掉目录已不存在的登记 |
| 10 | 没持锁时怎么办？ | 不清理；等下次取锁的对账 |

**第 3 轮**

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 11 | 要不要手动清理入口？ | 要；Run 段操作位放 `Clean up…`，按第 2 节的表决定是否可用 |
| 12 | 手动清理要确认吗、强制吗？ | 确认框列出未提交文件、未合并提交和孤儿进程；确认后强制清理 |
| 13 | 手动清理的 API 和锁？ | `POST /api/runs/{task_id}/cleanup`；先取锁，孤儿要 `terminate_orphan`，与 Resume 一致 |
| 14 | 要不要"全部清理"？ | 不要 |
| 15 | 重新拾取时旧 worktree 脏了、或分支有未合并提交？ | 强制删除，先把旧分支头 SHA 记进 Run 记录；孤儿还活着就本轮不拾取 |
| 16 | 成功收尾时删不掉（比如 agent 留了未跟踪文件）？ | 按自动清理规则处理：保留、记原因、每轮重试、可手动清理 |
| 17 | 清理要写 ticket 评论吗？ | 不写；只写 Run 记录，并推 `run_changed` |
| 18 | 界面显示什么？ | Run 段加一行遗留物状态，Details 里已删的路径标 `(removed)`；卡片和总览不变 |

第 4 轮：前沿为空。

## Comments

### 2026-10-10 chuck

本批决策带来的约束：
- [[TASK-2vpdd]]：清理规则要覆盖 `.markplane/.runs/<TASK-id>.json`。删除 worktree 时 Run 记录保留；可考虑随 ticket 归档或删除一起清理。

### 2026-10-10 chuck

第二批决策带来的约束：
- [[TASK-exz6h]]：如需手动清理入口，放在详情面板 Run 段的操作位；worktree 路径已在 Run 段的 Details 折叠区显示。
- [[TASK-2nt8u]]：Dispatcher 发起的任何 worktree、`agent/*` 分支或 `.runs/` 清理，都必须先持有该仓库的 Dispatcher 锁。

### 2026-10-10 chuck

第三批决策带来的约束：
- [[TASK-pd8bj]]：Run 是 `interrupted`，或是 `failed` 但 ticket 仍在 active 类别时，不能清理它的 worktree 和分支；孤儿 agent 进程还活着时也不能删它的 worktree。
