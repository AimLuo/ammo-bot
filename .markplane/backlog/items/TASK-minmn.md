---
id: TASK-minmn
title: serve 关闭、中断识别与 Resume
status: done
priority: medium
type: feature
effort: large
epic: EPIC-ce5ps
plan: null
depends_on:
- TASK-ph3az
- TASK-mzkqb
- TASK-d7i23
blocks:
- TASK-55iug
- TASK-kkmrn
related: []
assignee: null
tags:
- ready-for-agent
position: b08
created: 2026-10-10
updated: 2026-10-11
---

# serve 关闭、中断识别与 Resume

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

serve 停止或崩溃后，被打断的 Run 清楚地显示出来，由人一键 Resume，从断点接着跑；两个 agent 永远不会同时改一个 worktree。

- 正常关闭（SIGINT / SIGTERM）：停止拾取，给每个 agent 进程组发 SIGTERM，最多等 5 秒再 SIGKILL；不写 ticket，Run record 保持非终态。
- 识别（只读、不写任何东西）：记录非终态、本 serve 内存里没有这个 Run、锁空闲 → `interrupted`；锁被他人持有 → `running elsewhere`。interrupted 不占 Slot；running elsewhere 无任何操作。
- 孤儿探测：最后一次调用的 PID 还在且命令行包含 `agent` 和记录的 worktree 路径。不自动杀；Run 段显示 "Previous agent process still running (pid N)"；Resume 需确认后先杀其进程组，否则 409 `orphan_running`。孤儿还活着的 ticket 本轮扫描跳过。
- 取锁时对账：ticket 已不在 active 或 assignee 不是 `dispatcher` 的非终态记录写 `stopped`；其余保持中断。
- Resume：`POST /api/runs/{task_id}/resume`（可带 `terminate_orphan`）与 `POST /api/runs/resume`（当前 Project 全部，按拾取顺序，Slot 用完即停并报告）。前置检查依次：取锁（409 `dispatcher_locked`）→ ticket 在 active 且 assignee 是 `dispatcher`（否则写 `stopped`，409 `not_resumable`）→ worktree 与分支都在（否则按失败转人工，原因 "worktree missing"）→ 孤儿已处理 → 有空闲 Slot（409 `no_free_slot`）。不打开 Scan switch。
- 续跑规则：最后一次调用已有结果 → 直接做后续转换；implement / fix 未结束且 `<configDir>/chats/<md5(workspace)>/<id>/meta.json` 存在 → `--resume` 原会话（相同 `--workspace` 与环境）；init 的 `session_id` 与请求不同 → 立即杀掉、记 `resume_mismatch`、改为新开会话；无会话 ID 或 meta.json 不在 → 同 Stage 新开会话；`merge-queued` → 按原入队时间回队；merge 正在跑 → 目标分支已包含该分支就直接成功收尾，否则排到队首（锁内先 abort 残留合并）。每次在同一个 Run 里追加 `trigger: resume` 的调用，重新计 Stage 超时。`resume-interrupted` prompt 模板（原型分支 `prototype/stage-prompts`）。
- 界面：卡片与 Run 段显示 `Interrupted · <stage>` / `running elsewhere`；Run 段操作位 "Resume"（有孤儿时确认框）；Slot 徽章旁 "Resume N interrupted"。

## Acceptance criteria

- [ ] 关闭 serve 时 agent 进程组被终止，ticket 未被改动，Run record 保持非终态
- [ ] 新 serve 启动后该 Run 显示为 interrupted 且不占 Slot；另一个 serve 持锁时显示为 running elsewhere 且无操作
- [ ] Resume 前置检查按顺序返回对应 409；ticket 已被人接手时写 `stopped`
- [ ] 续跑原会话、`resume_mismatch` 后新开会话、已完成步骤直接转换、merge-queued 回队、merge 中途中断后的恢复，各有假 agent 场景
- [ ] 孤儿还活着时 Resume 返回 `orphan_running`，带 `terminate_orphan` 时先杀再续跑
- [ ] "Resume N interrupted" 按拾取顺序恢复，Slot 用完即停并提示
- [ ] 前端 typecheck、lint、生产构建通过
