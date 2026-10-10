---
id: EPIC-ce5ps
title: Agent Dispatcher：并行实现、合并与转人工
status: now
priority: medium
started: null
target: null
related:
- EPIC-24iwg
tags:
- ready-for-agent
created: 2026-10-10
updated: 2026-10-10
---

# Agent Dispatcher：并行实现、合并与转人工

## Problem Statement

`/to-tickets` 产出的 Implementation ticket 带着 `ready-for-agent`，但要有人一张张手动开 agent 会话去实现、review、修复、合并。这些 ticket 的依赖关系已经写在 tracker 里，哪张可以开工是确定的。人却要守在终端前搬运：开 worktree、调用 `/implement`、读 review 结论、再开修复会话、合并、推送、改 ticket 状态。多张 ticket 本可以并行，实际只能串行地等人。

## Solution

`markplane serve` 里加一个 **Dispatcher**：

- **扫描与拾取**：每个 Project 有一个 Scan switch（每次 serve 启动都是关）。打开后每 15 秒扫描一次，拾取 `planned`、带 `ready-for-agent`、未被阻塞的 Implementation ticket，在全局并发上限（默认 20）内并行执行。
- **Run 与 Stage**：每张 ticket 一个 Run，在独立的 git worktree 里依次跑三个 Stage：implement（调用 `implement` 技能，末尾自带 code review，以 `REVIEW: clean|issues` 收尾）→ 有问题时一轮 fix → merge（合并回主工作区，同一工作树内串行）。成功后 ticket 改为 `done`、提交 ticket 文件，按设置推送。
- **转人工**：任何 Stage 失败，ticket 换上 `ready-for-human`、写一条失败评论，出现在 Needs You 里。人写评论并点 "Comment & continue"，就从失败的 Stage 接着跑。
- **重启**：serve 重启后不自动扫描、不自动恢复；被中断的 Run 显示出来，由人点 Resume。
- **界面**：卡片上显示 Run 状态和当前 Stage 用时；看板顶部有 Scan switch 和 Slot 徽章，徽章弹出跨 Project 的活动 Run 总览；详情面板的 Run 段显示 Stage 进度、失败原因、用量和操作；Settings 新增 Dispatcher 标签页。

## User Stories

### 开关与设置

1. As a developer, I want a per-Project Scan switch on the board header, so that I decide which Projects the Dispatcher may work on.
2. As a developer, I want every Scan switch to start off whenever the web server starts, so that a restart never starts spending without me.
3. As a developer, I want turning a Scan switch on to fail with a clear reason when the Project isn't a git repository, `agent` isn't on PATH, or another web server holds the Dispatcher lock, so that I know what to fix.
4. As a developer, I want turning a Scan switch off to stop new pickups but let running Runs finish, so that work in flight isn't wasted.
5. As a developer, I want push-after-merge and each Stage's model and timeout stored in the repo's shared config, so that collaborators see the same policy and it's reviewable in git.
6. As a developer, I want push-after-merge to default on, with this repo able to turn it off in its own config, so that typical repos ship merges while mine stays local.
7. As a developer, I want each Stage's model to default to `cursor-grok-4.5-high` and each Stage's timeout to default to 60 minutes, so that it works without configuration.
8. As a developer, I want the global concurrency limit stored per machine, not per repo, so that one machine's capacity isn't dictated by a shared file.
9. As a developer, I want a Dispatcher tab in Settings with a shared-per-Project card and a this-machine card, so that I can see which settings travel with the repo.
10. As a developer, I want the model fields to offer this machine's available models while still accepting free text, so that I can pick quickly yet use a model my account lists differently.
11. As a developer, I want a model not in my local list to warn rather than be rejected, so that a shared config written by someone else still loads.
12. As a developer, I want out-of-range timeouts or concurrency rejected with a validation error, so that I can't save nonsense.
13. As a developer, I want settings to take effect without restarting the web server, including edits arriving via `git pull`, so that changes apply immediately.
14. As a developer, I want a running Stage to keep the model and timeout it started with, so that edits don't change work in flight.
15. As a developer, I want lowering the concurrency limit to stop new pickups rather than kill Runs, so that nothing is lost.
16. As a developer, I want the Settings tab to state plainly that agents run with full permissions, so that I understand the risk before turning scanning on.

### 拾取

17. As a developer, I want the Dispatcher to pick up only `planned`, unblocked Implementation tickets tagged `ready-for-agent`, so that only tickets I've released are worked on.
18. As a developer, I want status checks to use status categories rather than literal names, so that Projects with custom workflows work.
19. As a developer, I want a picked-up ticket moved to the active status with assignee `dispatcher`, so that it isn't picked up twice and I can see who owns it.
20. As a developer, I want pickups to rotate across Projects and, within a Project, follow priority, then board position, then creation date, then ID, so that one large Project can't starve the others and urgent work goes first.
21. As a developer, I want a Run to hold one Slot from pickup until it ends, including while waiting to merge, so that the concurrency limit is honest.

### Run 与 Stage

22. As a developer, I want each Run to work in its own worktree under `.worktrees/<TASK-id>` on branch `agent/<TASK-id>`, based on the main working tree's current HEAD, so that parallel Runs never collide.
23. As a developer, I want the implement Stage to invoke the `implement` skill on the ticket and end with `REVIEW: clean|issues`, so that the Dispatcher can decide the next step mechanically.
24. As a developer, I want review findings posted as one Comment on the ticket, so that I can read them later even when the Run succeeds.
25. As a developer, I want `REVIEW: issues` to start exactly one fresh fix session in the same worktree that ends with `FIX: done|blocked`, so that fixes happen once without endless loops.
26. As a developer, I want an ambiguous ticket handled by the agent recording its assumptions as a Comment instead of asking, so that unattended Runs never hang on a question.
27. As a developer, I want an agent that cannot do the work to say so and end with `BLOCKED: <reason>`, so that the ticket comes to me without wasting a fix round.
28. As a developer, I want agents unable to change ticket status, tags or assignee and limited to appending Comments, so that only the Dispatcher moves tickets.
29. As a developer, I want agent Comments to land in the main working tree's ticket file, not the worktree's copy, so that I can see them even when a Run never merges.
30. As a developer, I want each agent to commit or remove all its changes before printing its final line, so that worktrees can be cleaned up afterwards.

### 合并

31. As a developer, I want only one merge at a time per working tree, even across Projects and web servers sharing it, so that merges never race in one index.
32. As a developer, I want the merge Stage to merge `agent/<TASK-id>` into whatever branch the main working tree has checked out, so that it lands where I'm working.
33. As a developer, I want the merge Stage to resolve conflicts itself and give up with `MERGE: failed` when it can't, restoring the working tree, so that a failed merge never leaves the tree half-merged.
34. As a developer, I want my own uncommitted changes in the main working tree never stashed, reset or overwritten by a merge, so that my work is safe.
35. As a developer, I want a successful merge to mark the ticket `done`, commit only that ticket's file with a `Refs:` trailer, and push if enabled, so that dependents unblock and history stays clean.
36. As a developer, I want a success Comment with the target branch, merge commit SHA and whether it was pushed, so that the ticket records where its code went.

### 失败与转人工

37. As a developer, I want a Stage counted as failed on non-zero exit, a missing or invalid verdict line, no new commits after implement, an unresolved merge, a merge blocked by my uncommitted changes, or a timeout, so that every way of going wrong reaches me.
38. As a developer, I want a failed Run to keep the ticket in progress, swap `ready-for-agent` for `ready-for-human`, keep its worktree, and add a Comment whose first line is the reason, so that it shows up in Needs You with an explanation.
39. As a developer, I want a failure to free its Slot immediately, so that other tickets can proceed.
40. As a developer, I want a toast when a Run fails in the Project I'm viewing, with a button to open the ticket, so that I notice without watching the board.
41. As a developer, I want the Dispatcher to stop a Run quietly at the next Stage boundary when I move the ticket out of progress or reassign it, so that taking over a ticket is just editing it.
42. As a developer, I want a confirmation before I change the status or assignee of a ticket with an active Run, explaining that the current Stage will finish first, so that I know what will happen.
43. As a developer, I want the Dispatcher to turn a Project's Scan switch off by itself after three agent start failures in a row, with the reason shown, so that a broken login or model doesn't burn through the queue.

### 恢复与继续

44. As a developer, I want Runs that were going when the web server stopped shown as interrupted on their cards and Run sections, so that I know what needs attention.
45. As a developer, I want interrupted Runs to hold no Slot, so that they don't block new work.
46. As a developer, I want a Resume button on an interrupted Run and a "Resume N interrupted" button next to the Slot badge, so that I can resume one or all of the current Project's Runs.
47. As a developer, I want Resume to continue the original agent session when possible and otherwise start a fresh session in the same Stage, so that progress is kept whenever it can be.
48. As a developer, I want a resume that silently started an empty session detected and replaced with a fresh session, so that a lost session doesn't masquerade as a continuation.
49. As a developer, I want an agent still running from before a crash shown, and terminated only after I confirm on Resume, so that two agents never edit one worktree.
50. As a developer, I want "Comment & continue" on a failed or interrupted Run to post my Comment and send it to the Stage that failed, so that I can unblock the agent with an instruction.
51. As a developer, I want `ready-for-human` removed the moment my continue is accepted, so that Needs You clears straight away.
52. As a developer, I want a continued Run to rejoin the normal Stage flow, so that implement still needs a verdict and merge still serializes.
53. As a developer, I want continue refused without writing anything when the lock is held elsewhere or no Slot is free, so that a rejected continue leaves no half-state.
54. As a developer, I want Resume and continue to stay the same Run with a new invocation recorded, so that the history reads as one journey.

### 清理

55. As a developer, I want a Run's worktree and merged branch removed automatically once its ticket is done or cancelled, so that leftovers don't pile up.
56. As a developer, I want automatic cleanup to skip worktrees with uncommitted changes and branches with unmerged commits, so that nothing is lost without my say.
57. As a developer, I want a "Clean up…" action listing exactly what will be deleted before forcing it, so that I can clear leftovers deliberately.
58. As a developer, I want the Dispatcher never to touch worktrees or branches it didn't record, so that my hand-made worktrees are safe.
59. As a developer, I want re-releasing a failed ticket to `planned` with `ready-for-agent` to start over from a fresh worktree, keeping the old branch head SHA in the record, so that I can retry cleanly yet still recover old work.

### 界面

60. As a developer, I want a Run badge on each card showing the current Stage and minutes spent, merge-queue position, stopping, interrupted or failed, so that the board shows what the Dispatcher is doing.
61. As a developer, I want a "Slots used/limit" badge next to the Scan switch that opens a list of active Runs across Projects, so that I can see all agent work from any board.
62. As a developer, I want the Run section to show Stage progress with each step's result, model, time and tokens, the failure reason, and a Details fold with paths and SHAs, so that I can diagnose a Run without opening files.
63. As a developer, I want earlier Runs of the ticket listed in a fold, so that retries are visible.
64. As a developer, I want Runs held by another web server shown as "running elsewhere" with no actions, so that I don't mistake them for interrupted ones.
65. As a developer, I want Run changes pushed to the UI live and everything refreshed after a WebSocket reconnect, so that the board never shows stale Run state.

### 单 Dispatcher

66. As a developer, I want only one web server at a time to act as Dispatcher for a Project across every worktree of the repository, so that two servers never pick up the same ticket.
67. As a developer, I want a second web server to stay fully usable for viewing and editing while refusing Dispatcher actions with who holds the lock, so that running two servers is safe.

## Implementation Decisions

### 归属与事实来源

- Dispatcher 跑在 `markplane serve` 进程内；"程序重启"即 serve 重启。
- 主工作区的 `.markplane/` 是 ticket 状态的唯一事实来源。CLI 新增环境变量 `MARKPLANE_ROOT`：设置后从该目录找 `.markplane/`，不再从当前目录向上找。Dispatcher 给每个 agent 子进程设 `MARKPLANE_ROOT=<主工作区>` 和 `MARKPLANE_AUTHOR=agent-<stage>`（作者规则见评论 Spec）。worktree 里的 `.markplane/` 只读，分支不提交 `.markplane/` 改动。
- 只有 Dispatcher 改 ticket 的状态、标签和 assignee；agent 只追加评论。
- 所有状态判断按 workflow 状态类别（planned / active / completed / cancelled），不写死状态名。

### 设置

| 设置 | 位置 | 默认 | 范围 |
| --- | --- | --- | --- |
| Scan switch | serve 内存，按 Project，不落盘 | 每次启动都是关 | 开 / 关 |
| 合并后推送 | `config.yaml` 的 `dispatcher.push_after_merge` | 开（本仓写关） | 布尔 |
| 各 Stage 模型 | `config.yaml` 的 `dispatcher.stages.<stage>.model` | `cursor-grok-4.5-high` | 非空、无空白、≤200 字符 |
| 各 Stage 超时 | `config.yaml` 的 `dispatcher.stages.<stage>.timeout_minutes` | 60 | 1–1440 |
| 全局并发上限 | 用户配置目录 `serve.json` 的 `dispatcher.concurrency` | 20 | 1–100 |

- 全部可省略，缺省即默认；未显式设置的项不写回文件。读主工作区的 `config.yaml`。
- 全部热生效；Stage 开跑时对模型和超时取快照。调低并发不杀 Run；关 Scan switch 只停拾取。
- 打开 Scan switch 前检查：Project 是 git 仓库、`agent` 在 PATH 上、能取到 Dispatcher 锁。
- 越界返回 400 `validation_error`（同现有 `/api/config`）。手改出非法值时该 Project 沿用上次有效配置并在 `serve.log` 记警告；`serve.json` 读不出就用默认值。
- API：`/api/config` 的 GET / PATCH 增加 `dispatcher` 段；Scan switch 用 Project 前缀下的新端点；并发上限和模型列表用全局端点（与 `/api/projects` 同级）。
- 模型列表来自 `agent models`：没有机器可读格式，按文本解析（设 `NO_COLOR=1`、去掉 ANSI 码、逐行匹配 `^(\S+) - (.+)$`、去掉 ` (default)` / ` (current)` 后缀），serve 内缓存约 10 分钟，失败时下拉为空、只能自由输入。
- 界面：Scan switch 在看板顶部（与 "Decision tickets" 同一行）；Settings 新增 Dispatcher 标签页，两张卡片："This Project (shared via config.yaml)" 放推送开关和三个 Stage 的模型、超时；"This machine (all Projects)" 放并发上限和 `In use n`。加一行安全说明。不新增 CLI 配置命令。

### Dispatcher 锁

- 每个"仓库里的 Project"一把 OS 建议锁，键 = git common dir 绝对路径 + Project 目录相对所在 worktree 顶层的路径。锁文件放在 `<git common dir>/markplane/dispatcher/` 下，按键命名，保持为空、不回收；旁边原子写一个同名 `.json` 记持有者信息（pid、主机名、serve URL、用户配置目录、Project 路径、取锁时间），只用于显示。
- 非阻塞 `try_lock`，不做过期检测，进程退出由内核释放。锁的 fd 不能被 agent 子进程继承。
- 打开 Scan switch，或在未持锁时启动任何 Stage（Resume、continue、Clean up）前取锁；Scan switch 开着或本 serve 里该 Project 还有活动 Run 时一直持有，两者都没有时释放。
- 持锁才能做：拾取、启动 Stage、Dispatcher 写 ticket、写 `.runs/`、增删 Dispatcher 的 worktree 和 `agent/*` 分支。人和普通 CLI / MCP 编辑不受影响。
- 冲突时第二个 serve 只读：打开 Scan switch、Resume、continue、Clean up 返回 409 `dispatcher_locked`，带持有者信息；提示文案说明谁持有、怎么处理，主机名不同时注明"在另一台机器上"。不提供强制接管。
- 同一 serve 里同一锁键被另一个 Project 条目持有（如两个 worktree 都注册了）时直接拒绝并指出是哪个 Project。每次扫描核对锁文件是否还是同一个文件，不一致就当作锁已丢失：关 Scan switch、停止拾取、告警。
- 每个 Project 的 Dispatcher 状态里能读到锁的情况（空闲 / 本 serve 持有 / 他人持有及持有者），用共享锁探测，不改变状态。
- 独立 clone（各自有 `.git`）上的重复拾取不处理，文档写明"一个仓库只在一个地方开扫描"。

### 拾取

- 每 15 秒扫描已打开 Scan switch 的 Project。候选：状态在 planned 类别、带 `ready-for-agent`、不被阻塞（复用现有"被阻塞"判定：依赖未处于 completed / cancelled 即阻塞）的 Implementation ticket。旧 Run 的孤儿 agent 还活着的 ticket 本轮跳过。
- 顺序：跨 Project 轮流（每轮每个 Project 取一个）；Project 内按 priority（critical → someday）→ position → created → ID。
- 拾取写入：状态改为 active 类别的第一个状态，assignee 改为 `dispatcher`。Run 从拾取到结束占一个 Slot，包括在 Merge queue 里排队；interrupted 或已转人工的 Run 不占。

### Run 生命周期

| 时机 | ticket 状态 | 标签 | assignee | 其他 |
| --- | --- | --- | --- | --- |
| 拾取 | active 类别首个 | 不变 | `dispatcher` | 占 Slot |
| 成功 | completed 类别首个 | 保留 `ready-for-agent` | 保留 | `dispatcher` 署名评论：目标分支、合并 SHA、是否已推送；只提交该 ticket 文件（按路径，带 `Refs: TASK-xxx`），再按设置推送 |
| 失败 | 保持 active | 去掉 `ready-for-agent`，加 `ready-for-human` | 保留 | `dispatcher` 署名评论，首行是一句话原因，含失败的 Stage 和会话 ID；立即释放 Slot；保留 worktree |
| serve 关闭 | 不变 | 不变 | 不变 | 不算失败，Run record 保持非终态 |
| 人改了 ticket | — | — | — | 每个 Stage 开跑前（含 Merge queue 排到后）重读；状态不在 active 或 assignee 不是 `dispatcher` 时静默停止：不再写 ticket、释放 Slot、Run 记 `stopped`、保留 worktree。不中途杀进程 |

- 中间状态（in-progress、失败标签、评论）只写进主工作区文件，不提交。
- 流程：implement → `REVIEW: clean` → merge；`REVIEW: issues` → 一轮 fix → merge；不再循环 review。
- worktree `.worktrees/<TASK-id>`，分支 `agent/<TASK-id>`，基点是拾取时主工作区当前分支的 HEAD。建 worktree 前先 `git worktree prune`。
- 失败判定（任一即该 Stage 失败）：1）agent 非零退出；2）implement 末行没有合法的 `REVIEW: clean|issues`；3）implement 结束后分支相对基点没有新提交；4）fix 末行不是 `FIX: done`；5）merge 末行不是 `MERGE: done`、合并后目标分支不包含该分支、或主工作区的未提交改动挡住合并（主工作区里本分支留下的未完成合并除外）；6）超过 Stage 超时（进程组 SIGTERM，10 秒后 SIGKILL）。
- 末行判定的取法：取最后一个 `assistant` 事件的文本，再取其最后一个非空行；不用 `result` 字段（它把多段文本直接拼接）。成功 = 退出码 0 且有 `result/success` 事件。
- 所有状态转换先写 Run record 再执行；成功收尾要幂等（已是 `done` 不再改，文件无改动不提交，已推送不再推送）。

### agent 调用契约

- 固定参数：`agent -p --output-format stream-json --trust --force --approve-mcps --sandbox disabled --workspace <工作目录规范化绝对路径> --model <模型快照> [--resume <session_id>] "<prompt>"`。不传 `--auto-review`、`--mode` / `--plan`、`-w/--worktree`、`--add-dir`、`--api-key`。不提供附加参数设置。
- 首行 `system/init` 事件的 `session_id` 立即写入 Run record。解析时忽略未知事件类型。
- 进程：cwd 为工作目录；stdin 接 `/dev/null`；每个 agent 单独建进程组；继承 serve 完整环境，只额外加 `MARKPLANE_ROOT` / `MARKPLANE_AUTHOR`；不改 `HOME` / `CURSOR_CONFIG_DIR`。认证用 serve 所属用户的 `agent login`，不在配置里放 API key。
- implement 用 `/implement <TASK-id>` 显式调用技能（该技能禁止模型自动调用）。
- `--resume` 前校验 `<configDir>/chats/<md5(workspace)>/<id>/meta.json` 存在，并用与首次完全相同的 `--workspace` 和环境；init 里的 `session_id` 与请求不同说明静默开了空会话：立即杀掉，记 `resume_mismatch`，改为新开会话。
- 每次 Stage 调用记 `usage`（四个 token 字段）、`duration_api_ms`、stderr 末尾；没有 `result` 事件时 usage 为 null。
- 不设 token 或金额上限；花费由并发上限 × Stage 超时 × 每 Run 最多 3 个 Stage 限制，硬上限交给 Cursor 账号的消费上限。
- 同一 Project 连续 3 次 Stage 调用在拿到 `system/init` 前就结束：自动关闭该 Project 的 Scan switch，内存里记原因，写 `serve.log`，推 `dispatcher_changed`，界面在开关旁显示原因；有一次拿到 init 即清零。阈值写死。

### 各 Stage 的 prompt（原型：分支 `prototype/stage-prompts`）

- 英文模板加公共片段：无人值守规则（从不提问、不起常驻或交互进程）、worktree 边界（implement / fix 只写自己的 worktree，git 只动 `agent/<TASK-id>`，不 push，不改 git config）、ticket 规则（只用 `markplane comment` 追加，正文不以 H1–H3 开头，用 ticket 的语言写），以及各 Stage 的末行约定（输出末行前先提交或清掉所有改动）。
- implement：`/implement <TASK-id>`；歧义时按最合理假设继续，review 前写一条 `**Assumptions**` 评论；做不了写 `**Blocked**` 评论并以 `BLOCKED: <原因>` 结尾（按失败判定 2 转人工）；不自修 review 发现，按 must-fix / optional 总写一条 `**Review**` 评论（clean 也写）；有 must-fix 即 `REVIEW: issues`。
- fix：Dispatcher 把最新一条 `agent-implement` 写的 `**Review**` 评论内联进 prompt，未内联时 agent 自己 `markplane show`；确信是误报可以不修，在 `**Fix report**` 里说明，仍算 `FIX: done`；难修就 `FIX: blocked`。
- merge：只在持有合并锁时运行；`--no-ff --no-commit` 合并，提交前验证；干净合并只做 build / typecheck，手工解决过冲突才跑全量测试并写 `**Merge notes**` 评论；禁止 stash / reset / clean / `git add -A`；`.markplane/` 下冲突保留主工作区版本；放弃时 `git merge --abort`；以 `MERGE: done|failed` 结尾；prompt 不提推送。
- 续跑模板：`resume-interrupted`（续原会话，或新开会话并说明 worktree 里可能已有做了一半的改动）和 `continue-with-comment`（内联人的评论、上次失败原因，重申该 Stage 末行约定），通过 Stage 约定槽在三个 Stage 间复用。

### Merge queue 与合并锁

- 串行键是 Project 所在的工作树（per-worktree git dir 的 realpath），不是 Project。本 serve 内每个工作树一条 FIFO Merge queue，按进入 `merge-queued` 的时间排。
- 跨 serve 用工作树级合并锁 `<git dir>/markplane/merge.lock`（非阻塞 `try_lock`、保持为空、不回收），旁边 `merge.json` 记持有者，只用于显示。被他人持有时留在队首、定期重试。
- 临界区：取锁 → 重读 ticket → merge Stage → 合并后校验 → 提交 ticket 文件 → 按设置推送 → 失败时复原 → 释放锁。释放前若工作树里有本 Run 留下的未完成合并（存在 `MERGE_HEAD`），Dispatcher 执行 `git merge --abort`；abort 失败只记录。
- 排队不计时、不设超时；merge Stage 超时从取锁开跑算起。人的 git 操作不受合并锁约束。

### serve 关闭、孤儿与恢复

- 正常关闭（SIGINT / SIGTERM）：停止拾取，给每个 agent 进程组发 SIGTERM，最多等 5 秒再 SIGKILL（在 `serve stop` 的 10 秒宽限内）；不写 ticket，Run record 保持非终态。
- 被中断的识别：记录非终态、本 serve 内存里没有这个 Run、且锁空闲 → `interrupted`；锁被他人持有 → `running elsewhere`。识别不写任何东西。
- 孤儿探测：最后一次调用的 PID 还在，且命令行包含 `agent` 和记录的 worktree 路径。不自动杀；Run 段显示 "Previous agent process still running (pid N)"；Resume / continue / Clean up 需确认后先杀其进程组，否则返回 409 `orphan_running`。
- 取锁时对账：本 serve 取得某 Project 的锁时检查其非终态记录：ticket 已不在 active 或 assignee 不是 `dispatcher` 的写 `stopped`；其余保持中断，不自动恢复。
- Resume：`POST /api/runs/{task_id}/resume`（可带 `terminate_orphan`）和 `POST /api/runs/resume`（当前 Project 全部，按拾取顺序，Slot 用完即停并 toast 报告）。前置检查依次：取锁（409 `dispatcher_locked`）→ ticket 在 active 且 assignee 是 `dispatcher`（否则写 `stopped`，409 `not_resumable`）→ worktree 和分支都在（否则按失败转人工，原因 "worktree missing"）→ 孤儿已处理 → 有空闲 Slot（409 `no_free_slot`）。不需要也不会打开 Scan switch。
- 从第一个未完成的步骤接着跑：最后一次调用已有结果 → 不跑 agent，直接做后续转换；implement / fix 未结束且能续跑 → `--resume` 原会话；没有会话 ID 或 meta.json 不在 → 同 Stage 新开会话；`merge-queued` → 按原入队时间回队；merge 正在跑 → 目标分支已包含该分支就直接成功收尾，否则排到队首（锁内先 abort 残留合并）。每次恢复在同一个 Run 里追加一条 `trigger: resume` 的调用，重新计 Stage 超时，之后按正常流程走。

### Comment & continue

- 适用：最新 Run 是 `failed` 或 `interrupted`，ticket 在 active，worktree 和分支都在。`stopped` / `succeeded` 不能 continue；要从头来就把 ticket 改回 `planned` 并加 `ready-for-agent`。
- 入口：`CommentComposer` 附加按钮位上的 "Comment & continue"，不适用时不显示，锁被他人持有或没有空闲 Slot 时禁用并提示原因；失败 Run 的 Run 段操作位另有 "Continue…"，滚动到评论框并聚焦。
- `POST /api/runs/{task_id}/continue`，请求体 `{ "text": "…" }`（空文本 400）：先做全部检查（锁、适用条件、Slot、孤儿），任一不满足返回 409 且什么都不写；然后以人的身份追加评论；同一次 ticket 写入里去掉 `ready-for-human`、加回 `ready-for-agent`、assignee 设为 `dispatcher`、状态保持 active；Run 状态改回 `running`，结束时间清空，追加一条 `trigger: continue` 的调用；启动 Stage，启动就失败则走正常失败流程。Dispatcher 不另写"已继续"评论。
- 评论发给失败的 Stage：判定 1 / 6 → 失败的那个 Stage；判定 2 / 3 → implement；判定 4 → fix；判定 5 → 先进 Merge queue，目标分支已包含该分支就直接成功收尾。能续跑用 `--resume` 并内联评论，否则同 Stage 新开会话。不让人选 Stage。
- 继续后回到正常流程（implement 仍判 verdict，`issues` 走一轮 fix，然后 merge）；continue fix 不算第二轮 review。再次失败照常转人工；次数不设上限，不自动重试；Run 段显示 "continued n×"。

### 清理

- 只清理 Run record 里登记的 worktree（按 realpath 与 `git worktree list` 比对）和 `agent/<TASK-id>` 分支；只看每张 ticket 最新一次 Run。所有清理先持有 Dispatcher 锁；不持锁时不清理。
- 自动清理条件：Run 是 `failed` / `stopped` 且 ticket 在 completed / cancelled 类别（或文件已不存在），或 Run 是 `succeeded` 但收尾时没删掉。时机：取锁时对账、持锁期间每次扫描、成功收尾时。
- worktree 用 `git worktree remove`（不加 `--force`），有改动或未跟踪文件就跳过并记 `uncommitted_changes`；分支在 Project 所在工作树里 `git branch -d`，有未合并提交就保留并记 `unmerged_commits`；记录的目录已不在 → `git worktree prune`；目录在但 git 不认 → 不删，记 `not_a_worktree`；孤儿还活着 → 跳过，记 `orphan_running`。同一跳过原因只在状态变化时写日志。
- 手动 "Clean up…"（Run 段操作位）：`POST /api/runs/{task_id}/cleanup`（可带 `terminate_orphan`）。可用条件：Run 不处于 running / merge-queued / stopping / running elsewhere / interrupted，且不是"failed 但 ticket 仍在 active"，且还有 worktree 或分支剩下。确认框列出 worktree 路径和未提交文件数、分支和未合并提交数及分支头 SHA、要杀的孤儿 pid；确认后 `git worktree remove --force`、`git branch -D`、必要时 prune。不做"全部清理"。
- 重新拾取（ticket 被改回 planned 并带 `ready-for-agent`）时强制删除旧 worktree 和分支，删除前把旧分支头 SHA 写进旧 Run；旧 Run 的孤儿还活着就本轮不拾取。
- 清理后在 Run record 上写清理字段；不写 ticket 评论；推 `run_changed`。

### Run 界面

- 新接口：`GET /api/runs`（Project 前缀下，每张有记录 ticket 最新 Run 的摘要：task_id、序号、显示状态、Stage、Stage 开始时间、Run 开始时间、排队位次、失败原因、结束时间）；`GET /api/dispatcher`（全局：并发上限、已用 Slot、运行中与排队数、按 Project 分组的活动 Run、各 Project 的 Scan switch 状态与自动关闭原因、锁状态）。`GET /api/runs/{task_id}` 已由评论 Spec 提供。
- 显示状态由服务端统一计算：`running`（附 Stage）、`merge-queued`、`interrupted`、`running elsewhere`、`failed`、`stopped`、`succeeded`。客户端只额外派生 stopping（Run 仍活动，但 ticket 已不在 active 或 assignee 不是 `dispatcher`）。
- RunBadge（卡片底部 assignee 行右侧；Needs You 行与 Dashboard Active Work 复用）：`implement · 12m` 等（当前 Stage 用时，分钟取整，`<1m`，全页 30 秒计时器），`merge queue #2 · 5m`，`Stopping · implement`，`Interrupted · fix`，`Run failed · merge`，`Run stopped`，`running elsewhere`；succeeded 不显示；failed / interrupted / stopped 只在 ticket 仍在 active 时显示。悬停提示给出 Run 序号、总用时、Stage 开始时间、模型与超时、调用次数。遵守 `prefers-reduced-motion`。标记不单独可点。
- Run 段（补全评论 Spec 的最小 Run 段）：头部 RunBadge、`Run #n`、开始时间、总用时、操作位（View sessions、Resume、Continue…、Clean up…）；Stage 进度 `implement → fix → merge`（fix 只在 `REVIEW: issues` 时出现），每步结果图标、模型、用时、`in / out` token；失败原因全文；遗留物状态行（`Worktree kept · 3 uncommitted files` 等）；Details 折叠区（worktree 路径带复制、分支、基点 SHA、目标分支、合并 SHA、是否推送、每次调用的用量与 Run 合计，已删路径标 `(removed)`）；"Earlier runs (n)" 折叠区；孤儿提示。
- 看板顶部：Scan switch（禁用时提示原因，含锁被他人持有、自动关闭原因）旁边是 `Slots 12/20` 徽章（含排队，占满时警示色），点开弹出总览：`running 9 · merge queue 3 · limit 20`，按 Project 分组（当前 Project 在前），每行 ID、标题、RunBadge，点击跳到对应 ticket；没有时 "No active runs"。有 interrupted Run 时徽章旁出现 "Resume N interrupted"。
- WebSocket：`run_changed`（发到该 Project 通道，负载 task_id、序号、显示状态、Stage；每次写 Run record 后发）使 `runs` 查询失效；`dispatcher_changed`（发到所有已建立的 Project 通道，无负载；Slot、显示状态、排队位次、并发上限、任一 Scan switch 变化时发）使 `dispatcher` 查询失效。不推心跳和 agent 输出。重连成功（非首次）时让全部查询失效。当前 Project 收到 failed 时弹 toast `TASK-xxx run failed at <stage>`，附 "Open"。
- 有活动 Run 的 ticket 被拖出 active 列、或在详情面板改状态或 assignee 时先弹确认框，说明当前 Stage 结束后停止、不杀进程、保留 worktree。改其他字段不确认。不提供"终止 Run"按钮。

### 文档

- Dispatcher 文档写明安全说明：以 serve 所属用户身份、无沙箱运行 agent（可联网、执行任意命令、读写该用户可访问的文件、自动批准 MCP、看到 serve 的全部环境变量）；ticket 正文和评论就是 prompt，只在可信仓库上开扫描；建议用专用用户 / 虚拟机 / 容器、`permissions.deny`、不在 serve 环境里放密钥、在 Cursor 账号设消费上限；agent 不 push、不改 ticket 状态只靠约定。另写明"一个仓库只在一个地方开扫描"。

## Testing Decisions

- 好的测试只验证外部行为：通过服务端 API 驱动，检查 ticket 文件、Run record、git 状态和 WebSocket 事件，不测内部状态机的私有细节。
- **服务端 API + 假 agent（主接缝）**：
  - Dispatcher 调用的 agent 可执行文件路径可被替换（测试注入，生产默认 `agent`）。假 agent 是一个脚本：按 stream-json 输出 init / assistant / result 事件，按测试场景在工作目录里提交、写评论、打印指定末行、非零退出、挂起直到超时或被杀。
  - 扫描由显式的 tick 钩子触发，不等 15 秒计时器；超时可设得很短。
  - 每个测试建一个真实的临时 git 仓库加 Project，沿用 serve 命令里进程内构建 app、发一次性 JSON 请求的写法（Project 前缀、注册表、config 测试为先例）。
  - 覆盖：拾取条件与跨 Project 轮转；happy path（clean → merge → done、只提交 ticket 文件、`Refs:` 尾注、推送开关）；issues → fix → merge；六条失败判定各自转人工的 ticket 写入；静默停止；Merge queue 串行与合并锁；主工作区脏时 merge 失败且不碰用户改动；serve 关闭后中断识别、Resume 的各分支（续跑、新开、`resume_mismatch`、已完成步骤直接转换）；continue 的检查顺序与"失败时什么都不写"；Dispatcher 锁冲突返回 409 与 `running elsewhere`；连续 3 次启动失败自动关 Scan switch；自动与手动清理的跳过规则；`run_changed` / `dispatcher_changed` 事件。
- **core 纯函数单元测试**：候选筛选与排序（含轮转）、stream-json 解析与末行判定、显示状态计算、`agent models` 文本解析，放在现有 core 测试旁边（现有"被阻塞"判定的测试为先例）。
- **CLI 集成测试**：`MARKPLANE_ROOT` 让 `markplane comment` 等写到指定 Project，沿用现有调用二进制的集成测试写法。
- **前端**：不引入测试框架；改动的界面代码必须通过 typecheck、lint 和生产构建。

## Out of Scope

- 接入 OpenAI Codex CLI；"Codex review" 就是 `implement` 技能末尾的 `code-review`。
- serve 重启后自动扫描或自动恢复 Run；收养仍在运行的孤儿 agent。
- agent 沙箱或按 Run 的权限配置；token / 金额上限；用量折算金额；检测 worktree 外写入；过滤环境变量里的密钥；限制谁写的 ticket 能触发拾取；可配置的启动失败阈值；Dispatcher 附加参数设置。
- 按机器覆盖各 Stage 模型；Dispatcher 专用 CLI 配置命令。
- 独立 clone 上的重复拾取保护；网络盘上锁的可靠性保证；强制接管他人的 Dispatcher 锁。
- 界面上终止正在运行的 agent；推送 agent 实时输出或心跳；桌面通知；单独的 Dispatcher 总览页，以及着陆页、侧栏、Project 切换器上的 Run 角标。
- 跨 Project 的"全部恢复"与着陆页中断提示；continue 时由人选 Stage、次数上限、自动重试；中断 Run 的"放弃"按钮。
- 同仓库多 Project 的 `agent/<TASK-id>` 分支名撞车；人的 git 操作受合并锁约束；Merge queue 排队超时。
- Run record 的保留期与回收；"全部清理"；跨 Project 的遗留物列表；清理没有 Run record 的 worktree 或分支；重新拾取时改名保留旧分支。
- 评论、Needs You、Run record 格式与只读接口、会话记录弹窗：属于评论 Spec。

## Further Notes

- 来自 Map「Agent Dispatcher 与任务卡片增强」[[MAP-8aavc]]。依据的决策：Run 生命周期与任务状态映射 [[TASK-k6d9y]]、agent CLI 无头调用契约 [[TASK-g73u8]]、Run 记录存放位置 [[TASK-2vpdd]]、Dispatcher 设置模型 [[TASK-cq846]]、Run 在界面上的呈现 [[TASK-exz6h]]、强制单 Dispatcher [[TASK-2nt8u]]、重启恢复与 comment and continue [[TASK-pd8bj]]、无人值守的权限与成本边界 [[TASK-kbyh2]]、同一工作树内多个 Project 的 merge 串行 [[TASK-a8246]]、遗留 worktree 与分支的清理 [[TASK-6hqkh]]、各 Stage 的 prompt [[TASK-6tbmz]]。
- 依赖评论 Spec 先完成：`CommentComposer` 的附加按钮位、`MARKPLANE_AUTHOR`、Run record 格式、`GET /api/runs/{task_id}`、最小 Run 段。
- prompt 原型在分支 `prototype/stage-prompts`（模板加 `run-stage.sh` 试跑脚本）；调研稿在分支 `research/agent-cli-contract`。原型中用 PATH 上的 `markplane` 包装脚本代替环境变量，并拒绝改 ticket 的子命令；正式实现以环境变量为准。
- 术语见 `GLOSSARY.md`：Dispatcher、Run、Stage、Slot、Merge queue、Interrupted Run、Resume、Continue、Run record、Scan switch、Dispatcher lock、Comment、Needs You。
