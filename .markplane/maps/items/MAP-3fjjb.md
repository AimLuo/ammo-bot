---
id: MAP-3fjjb
title: markplane serve 多项目切换、后台运行与 HTTPS
status: done
specs:
- EPIC-rm3zt
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

- 静态导出的前端能否在 /p/<id>/ 路径前缀下运行 [[TASK-rm5w6]]: 可行：后端在路由前改写 /p/<id>/ 前缀；前端用 useSyncExternalStore 读项目 id，链接统一走 ProjectLink/useProjectRouter 并用 ESLint 强制；原型分支 prototype/project-prefix-routing

- markplane 在 macOS/Linux 上把 serve 放到后台运行的做法 [[TASK-fixyf]]: 重新启动自身并用 process_group/setsid 脱离终端；flock 锁住的 serve.pid 做单实例与存活判断；SIGTERM + axum 优雅关闭；前台 serve 检测已有服务只读写本地文件、不走网络

- axum 怎么接 rustls 提供 HTTPS/wss，并用 rcgen 生成浏览器可接受的自签名证书 [[TASK-zaagz]]: axum-server 0.8（tls-rustls，只用 aws-lc-rs）；rcgen 签 ECDSA 叶子证书（SAN localhost/127.0.0.1/::1、EKU serverAuth、CA:FALSE、≤825 天）；macOS 钥匙串 / Linux NSS 信任，换证书需重新信任

- 一个 serve 进程如何管理多个 Project 的加载、文件监听与 WebSocket [[TASK-iy25u]]: 首次访问时建立运行态（sync_all + 常开监听器 + 独立广播通道）不淘汰；改写层把项目放进 Request extensions；404 project_not_found / 409 project_unavailable；注册表加锁读写不缓存；文档页全局；旧地址跳落地页

## Not yet specified

## Out of scope

- `markplane mcp`（按项目、stdio 启动，不受影响）。
- 局域网访问与访问鉴权（只服务本机）。
- 自建本地 CA（mkcert 式）证书方案。
- Windows 上的后台运行与证书信任（Windows 只用前台 `markplane serve`）。
- 开机自启（launchd / systemd）：本次不做，以后需要时另开新地图。
