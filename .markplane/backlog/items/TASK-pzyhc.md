---
id: TASK-pzyhc
title: 按 /p/<id>/ 打开已注册的 Project
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-rm3zt
plan: null
depends_on:
- TASK-d5syu
blocks:
- TASK-vnshg
- TASK-y7nci
- TASK-jn5ka
related: []
assignee: chuck
tags:
- ready-for-agent
position: aY
created: 2026-10-10
updated: 2026-10-10
---

# 按 /p/<id>/ 打开已注册的 Project

## Parent

markplane serve：多 Project 切换、后台运行与 HTTPS [[EPIC-rm3zt]]

## What to build

主干：一个 `serve` 进程按 `/p/<id>/` 服务 Project registry 中的多个 Project。

- 在 Project 目录里运行 `markplane serve`：当前目录写入用户配置目录下的 `projects.json`（遵守 `XDG_CONFIG_HOME`；加锁读写、先写临时文件再 rename；realpath 去重，重复添加只更新 `lastOpened`；id 为目录名 slug，冲突加随机后缀），然后浏览器打开 `/p/<id>/`。不在 Project 目录里运行时打开 `/`。
- 请求改写层解析 `/p/<id>/...` 与 `/api/p/<id>/...`（含 `/api/p/<id>/ws`），按需建立该 Project 的运行态：首次 `sync_all`、只监视该 Project `.markplane/` 的常开监听器、独立的 WebSocket 广播通道；不淘汰。`lastOpened` 只在运行态建立时更新。
- 错误：id 不在注册表时重读一次注册表，仍找不到返回 404 `project_not_found`；路径不存在、`.markplane/` 被删或首次 sync 失败返回 409 `project_unavailable`（附路径）并释放运行态。
- 文档接口 `/api/docs` 留在全局；监听器不再监视仓库 `docs/`。不带前缀的旧页面地址 302 到 `/`。
- 前端最少改动：用 `useSyncExternalStore` 读当前 Project id（服务端快照 `null`，见 Epic 中原型 hook）；API 客户端统一加 `/api/p/<id>/` 前缀；WebSocket 连接带前缀（含开发模式的 `NEXT_PUBLIC_WS_URL`）；侧边栏链接带前缀、高亮判断先剥前缀；`next dev` rewrites 增加 `/p/:id/:path*` → `/:path*`。
- `/` 暂时可以只是一个最简单的 Project 列表（链接到 `/p/<id>/`），完整落地页在后续票。

## Acceptance criteria

- [ ] 在 Project 目录运行 `markplane serve` 后，`projects.json` 出现该 Project 且浏览器打开 `/p/<id>/`
- [ ] 两个 Project 在两个标签页同时打开，数据与 WebSocket 事件互不串扰（进程内测试覆盖）
- [ ] 404 `project_not_found` 与 409 `project_unavailable` 有进程内测试覆盖
- [ ] 同一目录（含经由符号链接）重复注册只保留一条
- [ ] `/api/docs` 不带前缀可用；`/backlog/` 302 到 `/`
- [ ] 刷新任意 `/p/<id>/...` 页面、侧边栏跳转正常；`next dev` 下同样可用
