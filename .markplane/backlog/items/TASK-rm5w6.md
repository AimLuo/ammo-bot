---
id: TASK-rm5w6
title: 静态导出的前端能否在 /p/<id>/ 路径前缀下运行
status: done
priority: medium
type: prototype
effort: medium
epic: null
plan: null
map: MAP-3fjjb
depends_on: []
blocks: []
related: []
assignee: chuck
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

## Answer

可行，维持 `/p/<id>/...` 路径前缀方案（不改用 `?project=` 查询参数）。原型：分支 `prototype/project-prefix-routing`（`cdb5e06`，仅本地），用无头 Chrome 在正式模式与 `next dev` 下实测，无控制台报错。

结论：

- **后端**：在整个 Router 外面套 `tower::util::MapRequestLayer`，在路由匹配之前把 `/p/<id>/x` 改写为 `/x`、`/api/p/<id>/x` 改写为 `/api/x`、`/api/p/<id>/ws` 改写为 `/ws`，再用 `axum::ServiceExt::into_make_service` 启动。现有路由与嵌入式静态文件服务不用改；`_next/` 资源是绝对路径，不受前缀影响。（正式实现中改写层负责解析 `<id>` 并选出对应 Project，而不是丢弃。）
- **已验证正常**：直接打开 / 刷新带前缀的页面、侧边栏与命令面板跳转（软跳转）、浏览器后退、Next 预取（自动带前缀）、带前缀的 API 与 WebSocket 升级。
- **前端 API**：所有接口都经过 `lib/api.ts`，在这一处统一加上 `/api/p/<id>/` 前缀。
- **项目 id 的读取**：静态导出在构建时预渲染，此时没有项目 id；hydration 后 React 不会修正 `href` 属性，导致 Cmd 加点击 / 新标签页打开会丢失项目。必须用 `useSyncExternalStore` 写的 hook 读取（服务端快照返回 `null`，挂载后再换成真实 id），不能在渲染时直接读 `window`。
- **链接**：约 30 处 `<Link>` / `router.push` 统一改走 `ProjectLink` / `useProjectRouter`；用 ESLint `no-restricted-imports` 禁止直接引入 `next/link` 与 `useRouter`，防止漏改（原型中未改造的 Maps 列表链接点击后会丢失前缀）。
- **当前位置判断**：侧边栏高亮这类 `pathname.startsWith(...)` 判断，要先剥掉 `/p/<id>` 再比较。
- **开发模式**：`next.config.ts` 加一条 rewrite（`/p/:id/:path*` → `/:path*`），API 按现有方式代理到 Rust；`NEXT_PUBLIC_WS_URL` 目前写死为 `ws://localhost:4200/ws`，需要改为带项目前缀。
