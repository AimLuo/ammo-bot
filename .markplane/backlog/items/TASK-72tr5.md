---
id: TASK-72tr5
title: Dispatcher 锁与 Scan switch
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-ce5ps
plan: null
depends_on: []
blocks:
- TASK-cppqm
related: []
assignee: null
tags:
- ready-for-agent
position: aw
created: 2026-10-10
updated: 2026-10-11
---

# Dispatcher 锁与 Scan switch

## Parent

Agent Dispatcher：并行实现、合并与转人工 [[EPIC-ce5ps]]

## What to build

看板顶部的 Scan switch，以及保证一个 Project 同时只有一个 serve 当 Dispatcher 的 Dispatcher lock。本票只管开关与锁，打开后暂不拾取（拾取在主干票）。

- Dispatcher lock：每个"仓库里的 Project"一把 OS 建议锁，键 = git common dir 绝对路径 + Project 目录相对所在 worktree 顶层的路径；锁文件在 `<git common dir>/markplane/dispatcher/` 下按键命名、保持为空、不回收，旁边原子写同名 `.json` 记持有者（pid、主机名、serve URL、用户配置目录、Project 路径、取锁时间）。非阻塞 `try_lock`，不做过期检测；锁 fd 不能被子进程继承。持有规则：Scan switch 开着或本 serve 里该 Project 还有活动 Run 时持有，两者都没有时释放（本票只有前者）。
- 同一 serve 里同一锁键已被另一个 Project 条目持有时直接拒绝并指出是哪个 Project。锁状态（空闲 / 本 serve 持有 / 他人持有及持有者）用共享锁探测，不改变状态。
- Scan switch：serve 内存、按 Project、每次启动都是关。打开前检查 Project 是 git 仓库、`agent` 在 PATH 上（可执行文件路径可由测试注入）、能取到锁；失败给出明确原因，锁冲突返回 409 `dispatcher_locked` 带持有者信息（主机名不同注明"在另一台机器上"）。关闭只停拾取。
- API：Project 前缀下的 Scan switch 端点；全局 `GET /api/dispatcher`，本票返回各 Project 的 Scan switch 状态与锁状态，后续票补全 Slot 与活动 Run。WebSocket `dispatcher_changed`（发到所有已建立的 Project 通道、无负载），Scan switch 变化时发；前端收到后让 `dispatcher` 查询失效。
- 界面：Scan switch 在看板顶部与 "Decision tickets" 同一行；禁用或打开失败时提示原因（含锁被他人持有）。第二个 serve 仍可正常查看与编辑。

## Acceptance criteria

- [ ] 非 git 仓库、`agent` 不在 PATH 上时打开开关失败并给出原因
- [ ] 两个 serve（进程内两个 app 实例即可）对同一 Project：第二个打开开关返回 409 `dispatcher_locked` 及持有者信息；第一个关闭后第二个可以打开
- [ ] 同一 serve 注册同一仓库同一 Project 的两个 worktree，第二个被拒绝并指出冲突 Project
- [ ] serve 重启后开关是关
- [ ] `GET /api/dispatcher` 返回开关与锁状态；切换开关推 `dispatcher_changed`
- [ ] 看板顶部开关可用并显示失败原因；前端 typecheck、lint、生产构建通过
