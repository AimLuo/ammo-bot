---
id: TASK-d5syu
title: 预重构：抽出 Router 工厂，处理函数从请求扩展字段取 Project
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-rm3zt
plan: null
depends_on: []
blocks:
- TASK-pzyhc
- TASK-kcivc
related: []
assignee: chuck
tags:
- ready-for-agent
position: aX
created: 2026-10-10
updated: 2026-10-10
---

# 预重构：抽出 Router 工厂，处理函数从请求扩展字段取 Project

## Parent

markplane serve：多 Project 切换、后台运行与 HTTPS [[EPIC-rm3zt]]

## What to build

预重构，对外行为不变（仍只服务启动目录这一个 Project）。把组装 Router 的逻辑从 `serve` 的启动流程里抽成一个工厂，便于进程内测试；在整个 Router 外套一层请求改写（在路由匹配之前运行，见 Epic 中“路由与 API 契约”），当前先把唯一的 Project 注入 Request extensions；所有处理函数改为从扩展字段取 Project，不再从全局 state 取。

同时建立第一个测试切入点：进程内构造应用、用 `tower::ServiceExt::oneshot` 发请求，不起端口。

## Acceptance criteria

- [ ] 处理函数不再直接引用全局 state 中的 Project，全部从 Request extensions 取得
- [ ] 请求改写层在路由匹配之前运行，并以 `axum::ServiceExt::into_make_service` 方式启动
- [ ] 有进程内测试覆盖若干代表性接口（如 summary、tasks 列表与单条、config、WebSocket 握手），基于临时目录中的 Project
- [ ] `markplane serve` 与 `--dev` 的现有行为不变；`cargo test --workspace`、`cargo clippy --workspace` 无警告
