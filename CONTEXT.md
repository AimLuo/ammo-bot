# Ammo-bot

本地单用户的 Beads 操作台：给人用的看板，外加可选的 scanner，在已登记的 project 上对 ready 的 bead 跑 pipeline。

## 用语

### 产品

**Ammo-bot**：
操作台本身——board、scanner、pipeline 合为一个产品。
_Avoid_: Scotty（冻结的 vendor 底本）、Sandcastle（agent 编排库）、"the bot"

**Project**：
登记在 ammo-bot 里、带 Beads 库的 git 仓库。board 展示该 project 的 bead。implement 和 review 在该仓库的 **worktree** 里跑；merge 落在它的 **primary checkout**。
_Avoid_: repo、workspace、sandbox

**Primary checkout**：
project 登记 path 上的工作树——git 的主 worktree，不是 linked **worktree**。Merge 合到这里。
_Avoid_: 主工作区、host cwd、main worktree（会和名为 main 的 target branch 混淆）

**Board**：
某个 project 的 bead 看板 UI。
_Avoid_: Scotty、dashboard、UI（太泛）

### 队列

**Bead**：
Beads 里的工作项。scanner claim 的单位，也是 pipeline 跑的单位。epic 不是 pipeline 要执行的 bead。
_Avoid_: task（Sandcastle 的词）、ticket、job、work item

**Ready**：
`bd ready` 会返回的 bead：open、无 blocker、不是 in_progress。
_Avoid_: open（太宽）、backlog

**Unattended queue**：
已 **Armed** 的 scanner 会考虑的 bead——**ready** 去掉 wayfinder，再去掉带 **human flag** 的。
_Avoid_: ready queue（那份含 wayfinder 和待处理的 Needs You）

**Armed**：
按 project 的模式。Armed 期间，该 project 的 scanner 可以从 unattended queue claim。默认未 Armed。Disarm 停止新的 claim；已在飞的 pipeline 跑完。
_Avoid_: enabled、auto、unattended、running

**Claim**：
原子领取一张 bead（`bd update --claim`）：status 为 `in_progress`，assignee 为领取的 actor。board 上的人和 scanner 都可以 claim；输掉的一方跳过。
_Avoid_: assign、start、grab、不带 assignee 的 `setStatus` 到 in_progress

**Human actor**：
board 写 Beads 时用的 actor 名。
_Avoid_: user、operator

**Agent actor**：
scanner 和 pipeline 在 claim 和写入时用的 Beads actor 名。固定为 `ammo-bot`。与 **human actor** 不同。
_Avoid_: bot user、cursor、BEADS_ACTOR

### 执行

**Scanner**：
对已 Armed 的 project，从 **unattended queue** claim，并为每张 claim 到的 bead 启动一条 **pipeline** 的循环。
_Avoid_: 扫描执行、worker、daemon、orchestrator

**Pipeline**：
一张已 claim 的 bead，走 implement → review → merge → close。implement 是 **iteration** 循环（上限 100）。review 是一次 **iteration**。merge 是 **primary checkout** 上的一次 **merger** **iteration**（上限 1，与 Sandcastle 的 merger 相同），受 **merge lock** 保护。**human flag** 一出现（agent 或人打上的都算），立刻停掉当前 Cursor **iteration**；pipeline 进程不退出，等人摘标后 **Resume**。Ammo-bot 自己启动 Cursor CLI，不调用 Sandcastle 的 `run()`。
_Avoid_: run、job、"sandcastle run"

**Iteration**：
对一张 **bead** 调用一次 Cursor CLI，最多产生一个 commit。Cursor 不能 resume session；下一次 **iteration** 是新进程，靠 git 里已有的 commit 接上。
_Avoid_: run、cycle、loop、one-shot

**Completion signal**：
agent 输出里的 `<promise>COMPLETE</promise>`，表示本阶段没有更多 **iteration**。无 payload。
_Avoid_: done flag、exit signal

**Worktree**：
项目 `.worktrees/` 下的 git worktree，进行中的每条 pipeline 一个，由 ammo-bot 自动创建。路径 `.worktrees/ammo-issue-<id>`，分支 `ammo/issue-<id>`。implement 和 review 在这里进行。merge 不合到这里。成功 merge 后删除该 worktree 和 named branch；**Needs You** 时留下。
_Avoid_: sandbox、workspace、`.sandcastle/worktrees`、merge worktree

**Merger**：
拿着 **merge lock** 的 Cursor agent：把 `ammo/issue-<id>` 合进 **primary checkout** 上的 **target branch**，解冲突，跑 typecheck 和测试，并 close 这张 **bead**。一次 **iteration**，然后发 **completion signal**。
_Avoid_: 纯脚本 merge、批量 merger、把 merge 当成只跑 `git merge` 的壳步骤

**Merge lock**：
按 **target branch** 的锁，同一时刻只有一个 **merger** 写 **primary checkout**。其他 bead 的 implement / review 不拿这把锁。
_Avoid_: worktree lock、queue（那是 scanner 的事）

**Target branch**：
merge 合入 **primary checkout** 的那条分支。按 **project** 配置，默认 `main`。
_Avoid_: HEAD、current branch、base branch

**Origin**：
作者是人还是 agent：名字在 human allowlist 里就是人。卡片 badge 用。不是 **Needs You** 的进件条件。
_Avoid_: human flag、human label、Needs You

**Needs You**：
等人处理的看板收件箱：带 **human flag** 的 bead，外加已就绪的 **human-approval gate**。判定与 Scotty 相同。
_Avoid_: blocked（Beads 的 status）、inbox、"需要人工处理"、Origin

**Human flag**：
Beads 的 `human` 标签，和 Scotty **Needs You** 认的是同一标记。进入 **Needs You**，不改 status。谁打上的都算：pipeline 在 merge 无法进行时打上（**primary checkout** 不干净、不在 **target branch** 上，或 **merger** 未完成）；agent 在 **iteration** 中发现必须等人时自行打上并 comment 问题。标签一出现，当前 Cursor **iteration** 立刻停掉（不发 **completion signal**）。scanner 不领带该标签的 bead；进行中的 pipeline 进程不退出，人摘掉标签且不 close 才 **Resume**。不是 `bd human`（给人列出/回复/驳回），也不是 **Origin**。
_Avoid_: `bd human`（打标）、`bd human respond`（会 close，不是让 pipeline 继续）、Origin、gate、blocked、escalate

**Human-approval gate**：
等待人批准的独立 gate bead，就绪后进 **Needs You**。不是 **human flag**；merge 受阻不走 gate。
_Avoid_: human flag、human label

**Resume**：
摘掉 **human flag** 后，仍由 **agent actor** claim、**worktree** 还在的那条 **pipeline** 从暂停点继续——下一轮 implement、review 或 **merger** 的 prompt 带上这张 **bead** 的 comments。若仍无法进行，再打上 **human flag**。
_Avoid_: Resume merge、retry（太泛）、restart（像整条 pipeline 重来）
