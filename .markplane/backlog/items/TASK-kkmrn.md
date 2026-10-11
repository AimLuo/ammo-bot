---
id: TASK-kkmrn
title: 清理遗留 worktree 与分支
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-ce5ps
plan: null
depends_on:
- TASK-minmn
- TASK-d7i23
blocks: []
related: []
assignee: null
tags:
- ready-for-agent
position: b0A
created: 2026-10-10
updated: 2026-10-11
---

# 清理遗留 worktree 与分支

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

Run 留下的 worktree 和 `agent/*` 分支在不再需要时自动清掉，有风险的留着并说明原因；人也可以在看清会删什么之后强制清理；被重新放回队列的 ticket 从干净的 worktree 重新开始。

- 范围：只清理 Run record 登记的 worktree（按 realpath 与 `git worktree list` 比对）和 `agent/<TASK-id>` 分支，只看每张 ticket 最新一次 Run；先持有 Dispatcher 锁，不持锁不清理；绝不碰未登记的 worktree 或分支。
- 自动清理：Run 是 `failed` / `stopped` 且 ticket 在 completed / cancelled 类别（或文件已不存在），或 Run 是 `succeeded` 但收尾时没删掉。时机：取锁时对账、持锁期间每次扫描、成功收尾时。`git worktree remove`（不加 `--force`），有改动或未跟踪文件跳过记 `uncommitted_changes`；在 Project 所在工作树里 `git branch -d`，有未合并提交保留记 `unmerged_commits`；目录已不在 → `git worktree prune`；目录在但 git 不认 → 不删记 `not_a_worktree`；孤儿还活着 → 跳过记 `orphan_running`。同一跳过原因只在状态变化时写日志。
- 手动 "Clean up…"：Run 段操作位；`POST /api/runs/{task_id}/cleanup`（可带 `terminate_orphan`）。可用条件见父 Spec「清理」。确认框列出 worktree 路径和未提交文件数、分支和未合并提交数及分支头 SHA、要杀的孤儿 pid；确认后 `git worktree remove --force`、`git branch -D`、必要时 prune。
- 重新拾取：ticket 被改回 planned 并带 `ready-for-agent` 时，先把旧分支头 SHA 写进旧 Run，再强制删除旧 worktree 和分支；旧 Run 的孤儿还活着就本轮不拾取。
- 清理后在 Run record 写清理字段，不写 ticket 评论，推 `run_changed`；Run 段遗留物状态行与 Details 的 `(removed)` 随之更新。

## Acceptance criteria

- [ ] 失败 Run 的 ticket 被人改为 done / cancelled 后，干净的 worktree 与已合并分支被自动删除，Run record 记下清理字段
- [ ] 有未提交改动的 worktree、有未合并提交的分支、git 不认的目录、孤儿还活着的情况各自被跳过并记录原因
- [ ] 人手建的 worktree 与非 `agent/*` 分支从不被删除
- [ ] 手动 Clean up 的确认框内容准确，确认后强制删除；不满足可用条件时按钮不出现
- [ ] 重新放回 planned 的 ticket 被拾取时用全新 worktree，旧 Run 记有旧分支头 SHA
- [ ] 前端 typecheck、lint、生产构建通过
