---
id: TASK-iy25u
title: 一个 serve 进程如何管理多个 Project 的加载、文件监听与 WebSocket
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-3fjjb
depends_on: []
blocks: []
related: []
assignee: chuck
tags: []
position: aW
created: 2026-10-10
updated: 2026-10-10
---

# 一个 serve 进程如何管理多个 Project 的加载、文件监听与 WebSocket

## Question

一个 `serve` 进程服务 Project registry 中的多个 Project 时，后端如何管理每个 Project 的生命周期？需要定：

- `/p/<id>/` 改写层解析出 id 后，如何取得 `Project`：首次访问时懒加载并缓存，还是启动时加载注册表里的全部 Project；缓存是否淘汰。
- 首次打开某个 Project 时是否先执行 `sync_all`（现在单项目模式是启动时 sync）。
- 文件监听：按 Project 各开一个 watcher，何时开、何时关（有 WebSocket 连接时才开，还是加载后常开）。
- WebSocket 广播按 Project 隔离：每个 Project 一个 broadcast channel。
- id 不在注册表、路径失效或 `.markplane/` 被删时的响应（API 返回什么错误，前端如何显示）。
- 注册表 `projects.json` 与 CLI（`markplane serve` 自动注册）并发写入时如何避免互相覆盖。

## Answer

讨论确认（grilling，2026-10-10）。事实前提：`Project` 只是指向 `.markplane/` 的路径、无内存状态，每次请求读盘；“加载”的成本只在 `sync_all`、文件监听器与 WebSocket 广播通道。

1. **运行态的建立与释放**：每个 Project 在进程内有一份运行态（`Project` + 广播通道 + 文件监听器）。第一次访问 `/p/<id>/` 时才建立并缓存，不做淘汰；进程退出或该 Project 被移出 Project registry 时释放。
2. **首次同步**：建立运行态时先执行一次 `sync_all`；失败按第 5 条的“项目不可用”处理。
3. **文件监听器**：运行态建立时就开，一直开到运行态释放；不按 WebSocket 连接计数开关。只监视该 Project 的 `.markplane/`。
4. **请求取得项目**：路径改写层解析出 id、查到运行态，放进 Request extensions；处理函数改为从扩展字段取项目，不再用 `state.project`（46 处机械替换）。WebSocket 订阅该 Project 自己的广播通道，事件互不串扰。
5. **错误响应**：
   - id 不在注册表：先重读一次注册表，仍找不到则返回 404 `project_not_found`。
   - 路径不存在、`.markplane/` 被删除或首次同步失败：返回 409 `project_unavailable`（附路径），并释放运行态，下次访问时重试。
   - 前端遇到这两种错误显示整页提示，提供“返回项目列表”。
6. **注册表读写**：服务端不在内存里长期缓存 `projects.json`；每次增、删、改名、列出都加锁读写（写临时文件再 rename），CLI 前台 `serve` 走同一套锁（见“markplane 在 macOS/Linux 上把 serve 放到后台运行的做法” [[TASK-fixyf]]）。`lastOpened` 只在运行态建立时更新一次。
7. **文档页**：展示的是 markplane 自身文档，与 Project 无关；`/api/docs` 留在全局，不放进 `/api/p/<id>/`；监听器不再监视仓库 `docs/`。
8. **旧地址**：不带前缀的页面地址（如 `/backlog/`）302 跳到落地页 `/`，由用户选择 Project。
