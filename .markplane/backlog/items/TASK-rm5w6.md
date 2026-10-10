---
id: TASK-rm5w6
title: 静态导出的前端能否在 /p/<id>/ 路径前缀下运行
status: draft
priority: medium
type: prototype
effort: medium
epic: null
plan: null
map: MAP-3fjjb
depends_on: []
blocks: []
related: []
assignee: null
tags: []
position: aV
created: 2026-10-10
updated: 2026-10-10
---

# 静态导出的前端能否在 /p/<id>/ 路径前缀下运行

## Question

Next.js 静态导出的前端能否在 `/p/<id>/...` 路径前缀下正常运行？用一个粗糙原型回答：

- Rust 服务端把 `/p/<id>/backlog/` 改写到 `backlog/index.html` 后，`_next/` 静态资源、客户端路由与刷新是否正常。
- 内部链接（`<Link>`、`router.push`、命令面板跳转）如何统一带上前缀：封装链接 helper，还是别的方式。
- 前端如何取得当前 Project id（`usePathname` 解析），切换 Project 时如何清空上一个 Project 的查询缓存。
- `next dev` 开发模式下如何做同样的改写（rewrites `/p/:id/:path*` → `/:path*`），以及 API / WebSocket 代理。
