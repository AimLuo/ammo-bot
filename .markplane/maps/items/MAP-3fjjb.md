---
id: MAP-3fjjb
title: markplane serve 多项目切换、后台运行与 HTTPS
status: active
specs: []
related: []
tags: []
created: 2026-10-10
updated: 2026-10-10
---

# markplane serve 多项目切换、后台运行与 HTTPS

## Destination

一份可直接交给 `/to-spec` 的 Epic：`markplane serve` 改为一个后台常驻进程服务 Project registry 中的多个 Project（落地页 + 切换），由 markplane 自己的命令负责后台运行（去掉 pm2），正式模式只开 HTTPS（自签名证书）。

## Notes

- 默认只规划不实现；地图完成后走 `/to-spec` → `/to-tickets`。
- grilling 工单调用 grilling + domain-modeling；术语见 `GLOSSARY.md`（Project、Project registry）。
- 语言：代码、UI 用英文；map、工单、讨论用中文。
- 参考实现：[bead-me-up-scotty](https://github.com/brendan-appstart/bead-me-up-scotty)（一进程多项目、`/p/<id>` 路由、服务端配置文件存列表、落地页 + 目录浏览弹窗、CLI 自动注册）。
- 现状：`serve` 启动时从当前目录锁定单个 Project，只监听 `127.0.0.1` 的 HTTP；前端为 Next.js 静态导出（不支持动态路由，现有页面用 `?map=` 之类 query 传 ID）；pm2 只托管开发模式下的 `next dev`（4201）。

已定的起点决策（charting 时确认）：

- 一个常驻进程同时服务多个 Project，切换不重启进程。
- 用户级文件统一放 `~/.config/markplane/`（遵守 `XDG_CONFIG_HOME`）：`projects.json`（`id`/`name`/`path`/`addedAt`/`lastOpened`）、`certs/`、`serve.pid`、`serve.log`。
- Project 即含 `.markplane/` 的目录；选中未初始化目录时提示并提供 init 入口。
- Project id：目录名 slug，重名加随机后缀；路径先 realpath，重复添加只更新 `lastOpened`。
- URL：页面用路径前缀 `/p/<id>/...`（服务端改写到静态页），API 统一 `/api/p/<id>/...`，WebSocket `/api/p/<id>/ws`。
- 落地页 `/`：卡片列出 Project（路径失效显示红点），按 `lastOpened` 倒序；添加用目录浏览弹窗（家目录起逐级点入或粘贴路径，支持 `~`），后端 `GET /api/fs?path=` 只列子目录并标出是否含 `.markplane/`，不限制浏览范围；支持移除（不删文件）和改显示名。
- 命令：`markplane serve` 保持前台；若后台已有进程则只注册当前目录并打开浏览器；新增 `markplane serve start|stop|status|logs`；默认端口 4200。
- pm2 全部删除（`ecosystem.config.cjs` 与 `pm2:*` 脚本），开发用 `npm run dev`。
- TLS：正式模式只开 HTTPS；`--dev` 保持 HTTP；提供 `--no-tls` 逃生口。
- 证书：`rcgen` 生成单张自签名证书，覆盖 `localhost` 与 `127.0.0.1`；首次启动打印信任命令，由用户自行决定是否信任。
- 平台：后台运行与证书信任只支持 macOS / Linux。

## Decisions so far

## Not yet specified

- 后端多 Project 的生命周期：何时加载、文件监听的开关与共享、首次打开是否先 sync、注册表与 CLI 并发写入。待原型与 Spec 阶段再定。
- 开机自启（launchd / systemd）：先不做，以后再看是否需要。

## Out of scope

- `markplane mcp`（按项目、stdio 启动，不受影响）。
- 局域网访问与访问鉴权（只服务本机）。
- 自建本地 CA（mkcert 式）证书方案。
- Windows 上的后台运行与证书信任（Windows 只用前台 `markplane serve`）。
