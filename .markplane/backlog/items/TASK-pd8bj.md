---
id: TASK-pd8bj
title: 重启恢复与 comment and continue
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-8aavc
depends_on:
- TASK-g73u8
- TASK-k6d9y
- TASK-2vpdd
blocks:
- TASK-6hqkh
related: []
assignee: chuck
tags: []
position: ak
created: 2026-10-10
updated: 2026-10-10
---

# 重启恢复与 comment and continue

## Question

serve 重启后的"恢复执行"和失败后的"comment and continue"各自确切做什么？

- 恢复执行：按钮在哪（全局 / 每个 Run）；中断的 Run 是 `--resume` 原会话续跑还是重跑当前 Stage；排队中的 merge 怎么办；恢复后扫描开关是否仍需单独打开。
- comment and continue：评论先写入 ticket 再作为 prompt 发给哪个 Stage 的会话；继续后 `ready-for-human` 何时移除；续跑成功后是否回到正常 Stage 流程（如 implement 续跑后仍判 verdict）。

## Answer

## Answer

### 代码现状（决策依据）

| 事实 | 位置 | 意义 |
| --- | --- | --- |
| serve 收到 Ctrl-C / SIGTERM 后，只对 HTTP 做 3 秒优雅关闭；`serve stop` 先发 SIGTERM，10 秒后发 SIGKILL | `serve.rs` `shutdown_signal`，`serve_daemon.rs` `stop` / `STOP_GRACE` | Dispatcher 要在这段宽限期内自己处理 agent 子进程，否则 serve 会被强杀 |
| daemon 模式下 serve 用 `process_group(0)` + `setsid` 自成会话；前台模式和终端在同一个前台进程组 | `serve_daemon.rs` `start` | agent 子进程如果不单独建进程组：前台时 Ctrl-C 会直接打到 agent（退出码 130）；serve 被 SIGKILL 或崩溃后 agent 成为孤儿继续跑 |
| 目前没有代码启动 agent 子进程，也没有 `kill_on_drop` | 全仓 grep | 进程组、关闭时杀进程、孤儿检测都要新写 |
| Rust std 打开的文件默认带 `O_CLOEXEC` | — | Dispatcher 锁的 fd 不会被 agent 继承，所以孤儿 agent 不会一直占着锁（[[TASK-2nt8u]]）。实现时不能破坏这一点 |

### 1. serve 关闭与孤儿进程

- **启动 agent**：每个 agent 子进程单独建进程组（`process_group(0)`）。Run 记录的调用里除 PID 外，再存进程组 ID 和用到的 agent configDir。
- **正常关闭**（SIGINT / SIGTERM）：Dispatcher 停止拾取，给每个活动 agent 的进程组发 SIGTERM，最多等 5 秒（留在 `serve stop` 的 10 秒宽限之内），还没退出的发 SIGKILL。不写 ticket，也不把 Run 改成终态，跟 [[TASK-2vpdd]] 一致：崩溃和正常关闭走同一条路。
- **崩溃 / SIGKILL**：agent 可能还活着。serve 启动时对每个非终态 Run 的最后一次调用做孤儿探测：PID 还在，**并且**它的命令行包含 `agent` 和记录的 worktree 路径（防止 PID 被复用），就算孤儿。探测只读，不需要锁。
- **处理方式**：不自动杀。Run 段显示 "Previous agent process still running (pid N)"；点"恢复"时弹窗确认后，先杀掉它的进程组（SIGTERM，5 秒后 SIGKILL）再恢复。没确认就返回 409 `orphan_running`。理由：serve 读不到孤儿的输出，没法判定它的结果；而且它和续跑的会话改的是同一个 worktree。
- 不收养孤儿，也不把 agent 输出改写到文件以便重启后接上（那等于自动恢复，在范围外）。

### 2. 被中断的 Run：识别、名额、收尾

- **识别**：沿用 [[TASK-2vpdd]] 和 [[TASK-2nt8u]]：记录是非终态（`running` / `merge-queued`），本 serve 内存里没有这个 Run，并且锁空闲（用共享锁探测）→ `interrupted`；锁被别的进程持有 → `running elsewhere`。识别本身不写任何东西。
- **名额**：被中断的 Run **不占名额**，确认 [[TASK-exz6h]] 的默认做法。恢复或 continue 时重新占一个名额；没有空闲名额就拒绝，返回 409 `no_free_slot`，Run 保持中断状态。
- **扫描开关**：恢复**不需要**先开扫描，也**不会**打开扫描；不记住重启前的开关状态，所以 `projects.json` 不新增字段。恢复和 continue 需要时自己取锁，Run 活动期间一直持有（[[TASK-2nt8u]]）。中断的 ticket 状态是 `in-progress`，不在 `planned`，所以开扫描也不会重复拾取它们。
- **取锁时对账**：不管是开扫描、恢复还是 continue 让本 serve 取得了某个 Project 的锁，都先过一遍它的非终态记录：ticket 已经不在 active 类别，或 assignee 已经不是 `dispatcher` 的，写 `stopped`（跟静默停止一样，保留 worktree）；其余保持中断，**不自动恢复**。
- **不提供"放弃"按钮**：人要接手就直接改 ticket（改状态或 assignee）；下次对账时这个 Run 会变成 `stopped`。

### 3. "恢复执行"

**入口**（两个预留位置都用上）

| 位置 | 按钮 | 作用范围 |
| --- | --- | --- |
| 详情面板 Run 段头部操作位 | `Resume` | 这一个 Run；只在显示状态是 `interrupted` 时出现 |
| 看板顶部名额徽章旁 | `Resume N interrupted` | **当前 Project** 全部被中断的 Run；N>0 时才出现。按拾取顺序逐个恢复，名额用完就停，并 toast 报告恢复了几个、剩下几个 |

API：`POST /api/runs/{task_id}/resume`（可带 `{ "terminate_orphan": true }`）和 `POST /api/runs/resume`（当前 Project 全部），都在 Project 前缀下。

**前置检查**（按顺序，任一不满足就拒绝，Run 保持原样）：取锁（409 `dispatcher_locked`）→ 重读 ticket，要求在 active 类别且 assignee 是 `dispatcher`（否则写 `stopped`，返回 409 `not_resumable`）→ worktree 和 `agent/<TASK-id>` 分支都在（否则按失败处理，见下）→ 孤儿已处理 → 有空闲名额。

**从哪里接着跑**：按 Run 记录里第一个没完成的步骤：

| 最后一次调用的情况 | 做法 |
| --- | --- |
| 已记下结束结果（退出码、末行判定），只是后面的转换没写 | 不再跑 agent，直接按正常流程转换（例如 `REVIEW: clean` → 进 merge 队列） |
| implement / fix 没结束，有会话 ID，且当前 configDir 下能找到 `chats/<md5(workspace)>/<id>/meta.json` | 用相同的 `--workspace` 执行 `--resume <id>`，发"被中断后继续"的 prompt，重申末行约定 |
| 没有会话 ID（在 init 行之前就中断了），或 meta.json 不存在 | 在同一个 worktree 里为同一个 Stage 新开会话，用原来的 Stage prompt，并说明 worktree 里可能已有做了一半的改动 |
| `merge-queued` | 按原来的入队时间回到本 Project 的 merge 队列 |
| merge 正在跑 | 先检查：目标分支已经包含该分支 → 直接做成功收尾，不跑 agent。否则排到 merge 队列**最前面**（主工作区可能留着做了一半的合并），轮到时按上面两行决定续跑原会话还是新开会话 |

- **续跑校验**：stream-json 首行 init 里的 `session_id` 跟请求的 ID 不同，说明静默开了一个空会话（[[TASK-g73u8]]）。这时立刻杀掉它，这次调用记 `resume_mismatch`，改为新开会话。
- 每次恢复都在**同一个 Run** 里追加一条调用（`trigger: resume`），Run 序号不变。新的调用重新计 Stage 超时。
- 恢复之后完全按正常流程走：verdict 判定、一轮 fix、merge 串行、六条失败判定都照旧。
- worktree 或分支不在了 → 按失败处理：留 `in-progress`，换成 `ready-for-human`，评论原因 "worktree missing"。
- **成功收尾要幂等**：ticket 已是 `done` 就不再改；ticket 文件没有改动就不再提交；已推送就不再推送。这样中断在收尾中途发生时也能安全重来。

### 4. comment and continue

**适用条件**：最新一次 Run 是 `failed` 或 `interrupted`；ticket 在 active 类别；worktree 和分支都在。`stopped`、`succeeded` 的 Run 不能 continue；要从头再来就把 ticket 改回 `planned` 并加 `ready-for-agent`（[[TASK-k6d9y]]）。

**入口**：评论框附加按钮位上的 `Comment & continue`（[[TASK-z97fe]]）。不满足条件时不显示；锁被别处持有或没有空闲名额时禁用，悬停提示说明原因。失败的 Run 在 Run 段操作位另放一个 `Continue…`，点了会滚动到评论框并聚焦，不另做一个入口。

**API**：`POST /api/runs/{task_id}/continue`，请求体 `{ "text": "…" }`，文字不能为空（400）。服务端按顺序：

1. 取锁、检查适用条件、检查名额、处理孤儿（只有 interrupted 的 Run 可能有）。任一不满足就返回 409，**什么都不写**。
2. 用 `add_comment` 追加人的评论（作者是 `$USER`，规则同 [[TASK-z97fe]]）。
3. 在同一次 ticket 写入里：移除 `ready-for-human`，加回 `ready-for-agent`，assignee 设成 `dispatcher`，状态保持 `in-progress`。**`ready-for-human` 在 continue 被接受的这一刻就移除**，Needs You 马上清掉这一项。
4. Run 状态从 `failed` 改回 `running`（不新开 Run）。失败原因保留在那次调用里，Run 的结束时间清空。追加一条 `trigger: continue` 的调用。
5. 启动 Stage。启动就失败（比如 agent 不可用）时走正常失败流程，重新加上 `ready-for-human`。

Dispatcher 不另写"已继续"的评论；人的评论和 Run 记录已经够了。

**评论发给哪个会话**：发给**失败（或被中断）的那个 Stage 的会话**。能续跑时用 `--resume` 把评论原文内联进 prompt，并重申该 Stage 的末行约定。不能续跑时，为同一个 Stage 新开会话，prompt 里包含原 Stage prompt、上次的失败原因和人的评论。不让人选 Stage。

| 失败判定 | 续跑的会话 |
| --- | --- |
| 1 非零退出 / 6 超时 | 失败的那个 Stage |
| 2 没有合法的 `REVIEW:` 行 / 3 没有新提交 | implement |
| 4 fix 末行不是 `FIX: done` | fix |
| 5 merge 冲突、合并后不包含该分支、主工作区有未提交改动 | merge：先进 merge 队列（占名额、串行）；如果目标分支已包含该分支，直接做成功收尾 |

**继续之后的流程**：回到正常的 Stage 流程，从续跑的那个 Stage 往后走：

- implement → 再判 `REVIEW: clean|issues` 和"相对基点有新提交"；`issues` → 新开一轮 fix → merge。
- fix → `FIX: done` → merge。continue fix 不算第二轮 review 循环。
- merge → 成功收尾：`done`，提交 ticket，按开关推送。

**再次失败**：跟第一次一样转人工（换成 `ready-for-human`，评论原因）。continue 次数不设上限，不自动重试；Run 段显示 "continued n×"。

**interrupted 的 Run 也能 comment and continue**：效果等于带着人的评论恢复，前置检查和续跑规则跟第 3 节一样。

### 对并行票的假设

- [[TASK-kbyh2]]：恢复和 continue 的会话沿用跟首次调用相同的 agent 标志与权限。如果该票定了用量 / 成本上限，续跑的调用计入同一个 Run，超限时按该票的规则处理，本票不另设上限。
- [[TASK-a8246]]：不管 merge 串行的键最后是 Project 还是主工作区，"中断的 merge 排到队首、`merge-queued` 按原入队时间回队、continue merge 先入队"这几条都按那个键的队列来执行。

### 自问自答（均按推荐采纳）

**第 1 轮**

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 1 | serve 正常关闭时，正在跑的 agent 要杀掉吗？ | 要。按进程组 SIGTERM，等 5 秒后 SIGKILL；不写 ticket，Run 保持非终态 |
| 2 | agent 要单独建进程组吗？ | 要。这样前台 Ctrl-C 不会直接打到 agent，杀的时候也能连同它启动的子进程一起杀掉 |
| 3 | 被中断的 Run 占名额吗？ | 不占；恢复或 continue 时重新占 |
| 4 | 恢复需要先开扫描、会顺带打开扫描、或记住重启前的开关吗？ | 都不；需要时自己取锁 |
| 5 | 恢复按钮放哪？ | 两处都放：Run 段放单个 `Resume`；名额徽章旁放当前 Project 的 `Resume N interrupted` |
| 6 | 中断的 Run 要不要有"放弃"按钮？ | 不要；人改 ticket 就行，取锁时对账成 `stopped` |

**第 2 轮**

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 7 | 恢复是 `--resume` 原会话还是重跑当前 Stage？ | 优先 `--resume` 原会话；没有会话 ID 或 meta.json 不存在时，在同一 worktree 里新开会话重跑这个 Stage |
| 8 | 最后一次调用其实已经结束了怎么办？ | 不再跑 agent，直接做后面的转换 |
| 9 | 怎么防止 `--resume` 静默开出空会话？ | 先查 meta.json；再比对 init 行的 `session_id`，不一致就杀掉，改为新开会话 |
| 10 | serve 崩溃后还活着的孤儿 agent 怎么办？ | 用 PID 加命令行（`agent` 和 worktree 路径）识别；只显示，不自动杀；恢复时确认后再杀 |
| 11 | 排队中的 merge 重启后怎么办？ | 按原入队时间回队；正在跑的 merge 排到队首，并先检查是否其实已经合并完成 |
| 12 | 恢复时 ticket 已被人改过，或 worktree 不见了？ | 被人改过 → `stopped`；worktree 不见了 → 按失败转人工 |
| 13 | 恢复算新 Run 还是同一个 Run？ | 同一个 Run，追加一条 `trigger: resume` 的调用 |

**第 3 轮**

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 14 | continue 用哪个端点，按什么顺序写？ | `POST /api/runs/{task_id}/continue`：全部检查通过后，先写评论，再改标签和 assignee，再启动 Stage；任一检查失败就什么都不写 |
| 15 | 评论发给哪个 Stage 的会话？ | 失败的那个 Stage；能续跑就 `--resume`，不能就新开会话；不让人选 |
| 16 | `ready-for-human` 什么时候移除？ | continue 被接受的那一刻，和加回 `ready-for-agent` 在同一次写入里 |
| 17 | 状态和 assignee 怎么变？ | 状态保持 `in-progress`，assignee 设成 `dispatcher` |
| 18 | 继续之后回到正常 Stage 流程吗？ | 回到。implement 仍要判 verdict，`issues` 走一轮 fix，然后 merge |
| 19 | continue 失败的 merge？ | 先进 merge 队列；已经合并完成就直接做成功收尾 |
| 20 | 再次失败怎么办？ | 照常转人工；不设次数上限，不自动重试 |
| 21 | 哪些 Run 能 continue？ | `failed` 和 `interrupted` 可以；`stopped` 和 `succeeded` 不行 |
| 22 | 失败的 Run 能不加评论直接重试吗？ | 不能，必须写评论；不加评论的恢复只给 interrupted 的 Run |
| 23 | Dispatcher 要另写"已继续"的评论吗？ | 不写 |

### 给后续票的约束

- [[TASK-6tbmz]]：需要两类额外的 prompt：implement / fix / merge 各一份"被中断后继续"，以及"带人工评论继续"（把评论原文内联，并重申末行约定）。新开会话的兜底 prompt 要说明 worktree 里可能已有做了一半的改动。merge prompt 要能处理主工作区里留下的、做了一半的本分支合并。
- [[TASK-6hqkh]]：Run 是 `interrupted`、或者是 `failed` 但 ticket 仍在 active 类别时，worktree 和分支不能清理，因为恢复和 continue 还要用。孤儿进程还活着时，不能删它的 worktree。
- [[TASK-by9nc]]：同一个会话 ID 下可能有多条调用（首次、resume、continue），要显示每条调用的触发方式；`resume_mismatch` 的调用要标出来。
- 实现层面：主工作区里**本分支**留下的未完成合并（`MERGE_HEAD` 等于 `agent/<TASK-id>` 的分支头），不算失败判定 5 里的"未提交改动挡住合并"。

## Comments

### 2026-10-10 chuck

本批决策带来的约束：
- [[TASK-2vpdd]]：重启时以 `.markplane/.runs/*.json` 里非终态的 Run（`running` / `merge-queued`）认定"被中断"的 Run；记录带会话 ID 和 agent PID，需决定孤儿 agent 进程怎么处理、被中断的 Run 是否占全局名额。
- [[TASK-cq846]]：serve 启动时所有 Project 的扫描开关都是关；如果"恢复执行"要记住重启前的开关，只能存进 `projects.json` 的 Project 条目，不能进 `config.yaml`。
- [[TASK-z97fe]]：comment and continue 复用评论框的额外按钮位；`POST /api/tasks/{id}/comments` 只追加、无副作用，"continue"要用单独端点或单独一步。

### 2026-10-10 chuck

第二批决策带来的约束：
- [[TASK-exz6h]]：卡片上的 `Interrupted · <stage>` 就是需要恢复的提示；"恢复执行"有两个预留位置：详情面板 Run 段头部的操作位，或看板顶部名额徽章旁。该票默认被中断的 Run 不占名额、不列入总览；若本票改为占名额，总览要把它们列出来。
- [[TASK-2nt8u]]：只有持有 Dispatcher 锁（或能取到锁）的 serve，才能把 `.runs/` 里非终态的 Run 判定为"被中断"并提供恢复。恢复执行和 comment and continue 启动 Stage 前，没持锁就先取锁，取不到就拒绝（409 `dispatcher_locked`）。
