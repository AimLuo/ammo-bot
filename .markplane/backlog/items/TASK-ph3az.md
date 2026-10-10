---
id: TASK-ph3az
title: 合并安全：跨 serve 合并锁、脏工作区保护与失败复原
status: planned
priority: medium
type: feature
effort: medium
epic: EPIC-ce5ps
plan: null
depends_on:
- TASK-in6ej
blocks:
- TASK-minmn
related: []
assignee: null
tags:
- ready-for-agent
position: b04
created: 2026-10-10
updated: 2026-10-10
---

# 合并安全：跨 serve 合并锁、脏工作区保护与失败复原

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

合并在任何情况下都不会互相踩踏、不会留下半合并的工作树、也不会碰人自己的未提交改动。

- 跨 serve 合并锁：工作树级 `<git dir>/markplane/merge.lock`（非阻塞 `try_lock`、保持为空、不回收），旁边 `merge.json` 记持有者，只用于显示。被他人持有时留在队首、定期重试。同一工作树里的多个 Project 共用一条 Merge queue。
- 临界区：取锁 → 重读 ticket → merge Stage → 合并后校验 → 提交 ticket 文件 → 按设置推送 → 失败时复原 → 释放锁。释放前若工作树里有本 Run 留下的未完成合并（存在 `MERGE_HEAD`）则 `git merge --abort`，abort 失败只记录。merge Stage 超时从取锁开跑算起；排队不计时。
- 主工作区的未提交改动挡住合并即判 merge 失败（主工作区里本分支留下的未完成合并除外），转人工；Dispatcher 与 merge prompt 都不 stash / reset / clean / `git add -A`，`.markplane/` 下冲突保留主工作区版本。
- merge prompt 补全：干净合并只做 build / typecheck，手工解决过冲突才跑全量测试并写 `**Merge notes**` 评论；放弃时 `git merge --abort` 并以 `MERGE: failed` 结尾。

## Acceptance criteria

- [ ] 两个 serve 实例共享同一工作树时，合并严格串行，另一方持锁期间本方 Run 停在队首并在释放后合并
- [ ] 同一工作树的两个 Project 的 Run 进同一条队列
- [ ] 主工作区有用户未提交改动且与合并冲突时，merge 失败转人工，用户改动逐字节不变
- [ ] 假 agent 留下未完成合并并输出 `MERGE: failed` 时，工作树被复原（无 `MERGE_HEAD`）
- [ ] merge 超时从取锁开始计
