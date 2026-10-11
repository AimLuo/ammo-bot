---
id: TASK-55iug
title: Comment & continue
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-ce5ps
plan: null
depends_on:
- TASK-minmn
- TASK-h2vcn
blocks: []
related: []
assignee: null
tags:
- ready-for-agent
position: b09
created: 2026-10-10
updated: 2026-10-11
---

# Comment & continue

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

人对失败或中断的 Run 写一条评论并点 "Comment & continue"，Run 就带着这条指示从失败的 Stage 接着跑，Needs You 立即清掉它。

- 适用：最新 Run 是 `failed` 或 `interrupted`，ticket 在 active，worktree 与分支都在。`stopped` / `succeeded` 不能 continue。
- `POST /api/runs/{task_id}/continue`，`{ "text" }`（空文本 400）：先做全部检查（锁、适用条件、Slot、孤儿），任一不满足返回 409 且什么都不写；然后以人的身份追加评论；同一次 ticket 写入里去掉 `ready-for-human`、加回 `ready-for-agent`、assignee 设为 `dispatcher`、状态保持 active；Run 改回 `running`、清空结束时间、追加 `trigger: continue` 的调用；启动 Stage，启动就失败则走正常失败流程。Dispatcher 不另写"已继续"评论。
- 发给哪个 Stage：失败判定 1 / 6 → 失败的那个 Stage；2 / 3 → implement；4 → fix；5 → 先进 Merge queue，目标分支已包含该分支就直接成功收尾。能续跑用 `--resume` 并内联评论，否则同 Stage 新开会话。`continue-with-comment` prompt 模板（内联人的评论、上次失败原因，重申该 Stage 末行约定）。
- 继续后回到正常流程（implement 仍判 verdict，`issues` 走一轮 fix，然后 merge；continue fix 不算第二轮 review）；再次失败照常转人工；次数不设上限。
- 界面：`CommentComposer` 附加按钮位上的 "Comment & continue"，不适用时不显示，锁被他人持有或没有空闲 Slot 时禁用并提示原因；失败 Run 的 Run 段操作位 "Continue…" 滚动到评论框并聚焦；Run 段显示 "continued n×"。

## Acceptance criteria

- [ ] 任一检查不满足时返回 409，ticket 文件与 Run record 均无变化
- [ ] continue 成功后：人的评论已追加、`ready-for-human` 换回 `ready-for-agent`、assignee 为 `dispatcher`、Run 为 running 并有 `trigger: continue` 调用
- [ ] 每类失败判定都被路由到规定的 Stage（假 agent 场景）；续跑 prompt 内联了评论
- [ ] continue 后 implement 以 `REVIEW: issues` 结束仍会走一轮 fix 再 merge
- [ ] "Comment & continue" 按钮的显示 / 禁用条件正确；"Continue…" 聚焦评论框；显示 "continued n×"
- [ ] 前端 typecheck、lint、生产构建通过
