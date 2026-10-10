---
id: TASK-a8246
title: 同一工作树内多个 Project 的 merge 串行
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-8aavc
depends_on: []
blocks: []
related: []
assignee: chuck
tags: []
position: ar
created: 2026-10-10
updated: 2026-10-10
---

# 同一工作树内多个 Project 的 merge 串行

## Question

"每个 Project 同一时刻只跑一个 merge Stage"（地图 Notes）默认一个工作树里只有一个 Project。monorepo 里两个 Project 共用同一个主工作区时，它们的 merge Stage 可能同时在同一工作树里合并。merge 串行的键是否改为主工作区（git 顶层目录）而不是 Project？排队的 merge 如何占全局名额，与 [[TASK-2nt8u]] 的 Dispatcher 锁（按 Project）如何配合？场景少见，可从简。

## Answer

## Answer

### 结论

merge 串行的键从 **Project** 改为 **工作树**（Project 目录所在的那个 git 工作树）。同一 serve 内，同一工作树的所有 Project 共用一条 **merge 队列**，按进入队列的先后排（FIFO）。跨 serve 用一把**工作树级合并锁**兜底：OS 建议锁，放在该工作树的 git dir 下。从 merge Stage 开跑一直持有到 ticket 文件提交、推送、失败复原全部结束。名额规则不变。Dispatcher 锁（[[TASK-2nt8u]]）的键不改。

### 已核实的代码事实

| 事实 | 位置 | 对本题的意义 |
| --- | --- | --- |
| 有 `.markplane/` 的任意目录都能注册成 Project，不要求是 git 顶层；CLI 向上找最近的 `.markplane/` | `serve_registry.rs` `register`、`serve.rs` `add_project`、`project.rs` `from_current_dir` | monorepo 子目录 Project 是合法场景：两个 Project 可以共用同一个工作树（同一个 index、同一个 HEAD） |
| 代码里还没有任何 git 调用 | 全仓 grep | 工作树的识别（`git rev-parse --show-toplevel` / `--absolute-git-dir`）要新写 |
| 同一仓库的多个 git worktree 各有自己的 index、HEAD 和 per-worktree git dir；git 默认不允许同一分支在两处同时检出 | `git worktree list`（本仓有 4 个） | 不同工作树里的 merge 落到不同分支、不同 index，互不冲突，不需要串行 |
| Dispatcher 锁的键 = git common dir + Project 相对工作树顶层的路径 | [[TASK-2nt8u]] | 同一工作树里的两个 Project 锁键不同，**可以被两个不同的 serve 分别持有**，所以只靠进程内队列挡不住 |
| merge 在主工作区进行，目标是合并时主工作区当前检出的分支；成功后在同一工作树按路径提交 ticket 文件，再按开关推送 | [[TASK-k6d9y]] | 合并、ticket 提交、推送都会写同一个 index / HEAD，必须在同一个临界区里 |

### 设计

| 项 | 决定 |
| --- | --- |
| 串行键 | 工作树 = 从 Project 目录算出的 per-worktree git dir（`git rev-parse --absolute-git-dir`，取 realpath；和 `--show-toplevel` 一一对应）。打开扫描开关时算一次，非 git 仓库本来就不能开扫描（[[TASK-cq846]]） |
| 进程内队列 | 本 serve 内每个工作树一条 FIFO 队列，按进入 `merge-queued` 的时间排；跨 Project 不轮转（拾取时的轮转只管拾取） |
| 跨进程锁 | 队首的 Run 再取 `<git dir>/markplane/merge.lock`（非阻塞 `try_lock`，保持为空、不回收），旁边原子写 `merge.json` 记持有者信息（pid、主机名、serve URL、Project 路径、TASK id、开始时间），**只用于显示**。被别的 serve 持有时留在队首、定期重试 |
| 临界区 | 取得锁 → 重读 ticket（[[TASK-k6d9y]] 的"排队结束重读"；已停止则立刻释放锁）→ merge Stage → 合并后校验 → 提交 ticket 文件 → 按开关推送 → 失败时复原工作树 → 释放锁 |
| 失败复原 | 释放锁前，如果工作树里有本 Run 留下的未完成合并（存在 `MERGE_HEAD`），Dispatcher 执行 `git merge --abort`；abort 本身失败就在失败评论和 `serve.log` 里写明，后面同一工作树的 merge 会按现有"未提交改动挡住合并"判定失败，不另做处理 |
| 名额 | 不变：每个 Run 一个名额，排队期间（包括在等别的 serve 释放锁）照占；工作树不另设名额 |
| 超时 | 排队不计时、不设排队超时；merge Stage 的超时（默认 60 分钟）从取得锁、开跑 Stage 才开始算 |
| 界面 | 不新增显示状态，仍然是 `merge-queued`；排队位次按工作树队列算（同一工作树的多个 Project 共用一套编号，所以总览里同一 Project 组内的位次可能不连续）。在等别的 serve 时位次显示 #1，悬停提示加 "waiting for another server"，附上 `merge.json` 里的持有者信息 |
| serve 退出或崩溃 | 锁由内核自动释放；被中断的 merge 怎么恢复归 [[TASK-pd8bj]] |
| 不纳入锁的操作 | 人自己在工作树里的 git 操作、普通 CLI / MCP 编辑、Run 拾取时的 worktree 创建、`.runs/` 写入（`.runs/` 不经过 git）；人的 git 操作和 merge 撞上时，按 [[TASK-k6d9y]] 现有的失败判定处理 |

### 场景核对

| 场景 | 结果 |
| --- | --- |
| 单 Project 的普通仓库 | 等同于原来的"每个 Project 一个 merge"，没有可见变化 |
| monorepo 里两个 Project，同一个 serve | 进程内同一条队列，FIFO |
| monorepo 里两个 Project，分属两个 serve（不同 `XDG_CONFIG_HOME`） | 允许（Dispatcher 锁各管各的）；由合并锁保证同一时刻只有一个 merge |
| 同一仓库的不同 git worktree 各自注册了不同的 Project | 工作树不同，各自串行、互不阻塞 |
| 同一 Project 出现在多个 worktree | 已经被 Dispatcher 锁挡住，只有一个 Dispatcher（[[TASK-2nt8u]]） |

### 自问自答（均按推荐采纳）

**第 1 轮**

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 1 | merge 串行的键用 Project 还是工作树？ | 工作树（Project 所在 git 工作树的 per-worktree git dir） |
| 2 | 同一工作树的两个 Project 由两个 serve 分别当 Dispatcher，允许吗？要不要把 Dispatcher 锁的键改成工作树？ | 允许；不改 [[TASK-2nt8u]] 的锁键，改用单独的合并锁兜底 |
| 3 | 同一仓库的不同 git worktree 之间要不要串行？ | 不要；index、HEAD、分支都是分开的 |

**第 2 轮**

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 4 | 跨进程靠什么保证？ | 工作树级 OS 建议锁 `<git dir>/markplane/merge.lock`，持有者信息写进旁边的 `merge.json`，做法同 Dispatcher 锁 |
| 5 | 同一工作树、不同 Project 的排队顺序？ | 进程内按进入队列时间 FIFO，不按 Project 轮转；队首再去取 OS 锁 |
| 6 | ticket 文件提交和推送要不要一起串行？ | 要；临界区从重读 ticket 到推送 / 失败复原结束 |
| 7 | 名额怎么算？ | 不变；排队（含等别的 serve）照占，工作树不另设名额 |

**第 3 轮**

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 8 | merge 失败后工作树处于合并中状态，会连带后续 merge 失败吗？ | 释放锁前由 Dispatcher `git merge --abort`；abort 失败只记录，不另做处理 |
| 9 | 排队要不要超时？Stage 超时从哪算？ | 排队不设超时；Stage 超时从取得锁后开跑算起 |
| 10 | 界面要不要新状态？ | 不要；仍是 `merge-queued`，位次按工作树算，在等别的 serve 时悬停提示附持有者 |
| 11 | 人的 git 操作要不要受合并锁约束？ | 不要；撞上时按现有失败判定处理 |
| 12 | 同一仓库的多个 Project 共用 `agent/<TASK-id>` 分支命名空间，ID 撞了怎么办？ | 不处理；5 位随机 ID 撞车的概率可以忽略 |

第 4 轮：前沿为空。

### 对并行票的假设

- [[TASK-pd8bj]]：假设恢复一个被中断的 merge 或 `merge-queued` 的 Run 时重新进入该工作树的队列、重新取合并锁；如果中断时留下了未完成的合并，在锁内先 abort 再重跑 merge Stage。
- [[TASK-kbyh2]]：假设 merge 会话本来就在主工作区运行、有权执行 git 合并和推送；本票不依赖它的结果。
