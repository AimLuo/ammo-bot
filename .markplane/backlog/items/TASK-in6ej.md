---
id: TASK-in6ej
title: Stage 失败转人工与静默停止
status: planned
priority: medium
type: feature
effort: medium
epic: EPIC-ce5ps
plan: null
depends_on:
- TASK-cppqm
blocks:
- TASK-gxxax
- TASK-ph3az
- TASK-rhzty
- TASK-d7i23
- TASK-gtgfz
related: []
assignee: null
tags:
- ready-for-agent
position: b01
created: 2026-10-10
updated: 2026-10-10
---

# Stage 失败转人工与静默停止

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

任何 Stage 出错都会把 ticket 交给人，并说明原因；人接手 ticket 时 Dispatcher 安静退出。

- 失败判定（任一即该 Stage 失败）：agent 非零退出；implement 末行不是合法的 `REVIEW: clean|issues`（含 `BLOCKED: <原因>`）；implement 后分支相对基点没有新提交；fix 末行不是 `FIX: done`（fix Stage 由后续票接入，判定先就位）；merge 末行不是 `MERGE: done` 或合并后目标分支不包含该分支；超过 Stage 超时（进程组 SIGTERM，10 秒后 SIGKILL）。
- 失败写入：ticket 保持 active；去掉 `ready-for-agent`、加 `ready-for-human`；assignee 保留；`dispatcher` 署名评论，首行是一句话原因，含失败的 Stage 和会话 ID；保留 worktree；立即释放 Slot；Run 记 `failed` 与对应失败判定。
- 静默停止：每个 Stage 开跑前（含 Merge queue 排到后）重读 ticket；状态不在 active 或 assignee 不是 `dispatcher` 时不再写 ticket、释放 Slot、Run 记 `stopped`、保留 worktree，不中途杀进程。

## Acceptance criteria

- [ ] 每条失败判定各有一个假 agent 场景，ticket 写入符合上述规则（标签互换、原因评论首行、assignee 与状态不变）
- [ ] 超时场景下 agent 进程组被终止，Run 记 timeout
- [ ] `BLOCKED: <原因>` 直接转人工，不进入 fix
- [ ] 失败后 Slot 立即释放，下一张候选可被拾取
- [ ] Stage 进行中人把 ticket 改出 active 或改 assignee，当前 Stage 结束后 Run 为 `stopped`，ticket 不再被 Dispatcher 修改
