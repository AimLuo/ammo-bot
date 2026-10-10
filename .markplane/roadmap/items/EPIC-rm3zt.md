---
id: EPIC-rm3zt
title: markplane serve：多 Project 切换、后台运行与 HTTPS
status: done
priority: medium
started: null
target: null
related: []
tags:
- ready-for-agent
created: 2026-10-10
updated: 2026-10-10
---

# markplane serve：多 Project 切换、后台运行与 HTTPS

## Problem Statement

我在本机同时维护多个用 markplane 管理的仓库。现在的 `markplane serve` 启动时锁定当前目录这一个 Project：想看另一个仓库，就得到那个目录再起一个进程、换一个端口；浏览器里没有项目列表，也没法切换。

服务也只能挂在前台终端里。前端开发时还要另外用 pm2 托管 `next dev`，正式运行却根本用不到 pm2，启动方式割裂。页面只走 HTTP，没有安全上下文。

## Solution

`markplane serve` 变成一个本机常驻的服务，同时服务 Project registry 中的多个 Project：

- **落地页与切换**：打开 `/` 看到 Project 卡片列表（按最近打开排序，路径失效显示红点）；用目录浏览弹窗或粘贴路径添加 Project；每个 Project 的页面都在 `/p/<id>/...` 下，书签稳定；侧边栏随时切换；可以移除（不删文件）和改显示名。选中尚未初始化的目录时提供 init 入口。
- **自动注册**：在某个 Project 目录里运行 `markplane serve`，这个目录会自动加入注册表并在浏览器打开；后台已有服务时直接复用。
- **后台运行**：`markplane serve start | stop | status | logs` 由 markplane 自己管理后台进程、pid 与日志；pm2 全部删除。
- **HTTPS**：正式模式只开 HTTPS，首次启动自动生成覆盖 `localhost` / `127.0.0.1` / `::1` 的自签名证书，并打印本机信任命令。

## User Stories

### Project registry 与落地页

1. As a developer, I want the web UI to open on a landing page listing my registered Projects, so that I choose which Project to work on before anything else.
2. As a developer, I want the landing page to show an empty state with an "add your first project" action when the registry is empty, so that a fresh install tells me what to do.
3. As a developer, I want to add a Project by browsing directories from my home folder in a dialog, so that I don't have to type paths.
4. As a developer, I want to paste a path (including `~`) into that dialog, so that I can jump straight to a deep directory.
5. As a developer, I want the directory browser to mark which folders contain `.markplane/`, so that I can spot Projects at a glance.
6. As a developer, I want selecting a folder without `.markplane/` to offer to initialize it, so that I can start tracking a new repo from the browser.
7. As a developer, I want adding the same directory twice (even via a symlink) to reuse the existing entry, so that the registry never holds duplicates.
8. As a developer, I want each Project to get a readable id derived from its directory name, with a random suffix only on collision, so that URLs stay recognizable.
9. As a developer, I want Projects ordered by when I last opened them, so that my active work is on top.
10. As a developer, I want a Project whose path has disappeared to show a red "unavailable" marker, so that I know which entries are stale.
11. As a developer, I want to remove a Project from the registry without touching its files, so that cleaning the list is safe.
12. As a developer, I want to rename a Project's display name, so that two repos with the same folder name are distinguishable.
13. As a developer, I want the registry stored in my user config directory (respecting `XDG_CONFIG_HOME`), so that it survives restarts and is shared by every `markplane` invocation.

### 在 Project 之间切换

14. As a developer, I want every page of a Project under `/p/<id>/...`, so that bookmarks and shared links always point to the right Project.
15. As a developer, I want a Project switcher in the sidebar, so that I can jump between Projects without returning to the landing page.
16. As a developer, I want switching Projects to clear the previous Project's cached data, so that I never see one Project's items inside another.
17. As a developer, I want opening a link in a new tab (Cmd-click, middle-click, copy link) to keep the current Project, so that multi-tab work behaves like normal web pages.
18. As a developer, I want live updates (file changes, config changes, sync) to arrive only for the Project I'm viewing, so that other Projects' activity doesn't disturb my view.
19. As a developer, I want two browser tabs on two different Projects to work at the same time against one server, so that I can compare Projects side by side.
20. As a developer, I want visiting an old unprefixed URL such as `/backlog/` to send me to the landing page, so that stale bookmarks fail gracefully.
21. As a developer, I want a clear full-page message with a "back to projects" action when a Project id is unknown or its directory is gone, so that I can recover without guessing.
22. As a developer, I want the bundled markplane documentation reachable regardless of which Project I'm in, so that docs aren't tied to any one repo.

### 命令行

23. As a developer, I want `markplane serve` run inside a Project to register that Project and open it in my browser, so that the old one-command workflow keeps working.
24. As a developer, I want `markplane serve` run outside any Project to open the landing page, so that I can still pick a Project.
25. As a developer, I want `markplane serve` to detect an already-running background server, register the current Project with it, open the browser, and exit, so that I never end up with two servers fighting over the port.
26. As a developer, I want `markplane serve start` to run the server in the background and return my terminal, so that the UI stays available after I close the shell.
27. As a developer, I want `markplane serve start` to report a clear error with the tail of the log and a hint (`--port`, `lsof`) when the port is taken, so that I can fix it quickly.
28. As a developer, I want `markplane serve stop` to shut the background server down gracefully and force it only after a timeout, so that in-flight requests finish.
29. As a developer, I want `markplane serve status` to tell me whether the server is running, its pid, port, URL and version, so that I can check it at a glance.
30. As a developer, I want `markplane serve logs` (with an optional follow mode) to show the background server's output, so that I can debug it without hunting for files.
31. As a developer, I want only one markplane server per user at a time, so that the registry, pid file and port never conflict.
32. As a developer, I want a stale pid file from a crashed server to be detected and ignored automatically, so that I never have to delete files by hand.
33. As a developer, I want the default port to stay 4200 with a `--port` override, so that existing habits still work.

### HTTPS

34. As a developer, I want the production server to serve HTTPS only, so that the UI runs in a secure browser context.
35. As a developer, I want a self-signed certificate generated automatically on first start, so that HTTPS works with zero setup.
36. As a developer, I want the certificate to be accepted by Safari, Chrome and Firefox once trusted, so that I'm not stuck with a browser that rejects it.
37. As a developer, I want the server to print the exact command that trusts the certificate on my OS (macOS keychain, Linux NSS for Chrome and Firefox), so that I can opt in without searching the web.
38. As a developer, I want the certificate renewed automatically when it is missing, near expiry, or no longer matches the expected hostnames, with a reminder to re-trust it, so that it never silently expires.
39. As a developer, I want the private key stored with owner-only permissions, so that other local users can't read it.
40. As a developer, I want a `--no-tls` escape hatch, so that I can fall back to HTTP if certificates get in my way.
41. As a developer, I want `--dev` mode to keep using HTTP, so that the Next.js dev server proxy keeps working.
42. As a developer, I want WebSocket live updates to work over `wss://`, so that HTTPS doesn't break real-time refresh.

### 前端开发

43. As a contributor, I want the pm2 process file and `pm2:*` scripts removed, so that there is a single obvious way to run the dev frontend (`npm run dev`).
44. As a contributor, I want the Next.js dev server to understand `/p/<id>/...` URLs and proxy prefixed API and WebSocket calls, so that dev mode matches production routing.
45. As a contributor, I want lint to fail when a component imports `next/link` or `useRouter` directly, so that no internal link can forget the Project prefix.

## Implementation Decisions

### 进程模型

- 一个 `serve` 进程同时服务多个 Project，切换不重启进程。默认端口 4200，`--port` 覆盖。
- 用户级文件统一放在 `~/.config/markplane/`（遵守 `XDG_CONFIG_HOME`）：`projects.json`、`certs/`、`serve.pid`、`serve.log`。

### Project registry

- 条目字段：`id`、`name`、`path`、`addedAt`、`lastOpened`。`id` 由目录名生成 slug，冲突时加随机后缀；`name` 默认为目录名，可改。
- 添加前对路径做 realpath；同一目录重复添加只更新 `lastOpened`。
- 合格的 Project 是含 `.markplane/` 的目录；不合格目录返回“未初始化”，由前端提供 init 入口。
- 服务端不在内存里长期缓存注册表：每次列出、添加、移除、改名都在加锁后读写文件；写入先写临时文件再 rename。CLI 前台 `serve` 走同一套锁。
- `lastOpened` 只在某个 Project 的运行态建立时更新一次，不是每个请求都写。
- 列表接口为每项附上路径是否有效。

### 路由与 API 契约

- 页面：`/` 为落地页；Project 页面统一在 `/p/<id>/...`。
- API：Project 范围的接口统一在 `/api/p/<id>/...`；WebSocket 在 `/api/p/<id>/ws`。
- 全局接口（不带 Project 前缀）：注册表（列表 / 添加 / 移除 / 改名）、目录浏览（`GET /api/fs?path=`，只列子目录、隐藏点开头的目录、标出是否含 `.markplane/`，不限制浏览范围）、init、文档（`/api/docs`）。
- 不带前缀的旧页面地址（如 `/backlog/`）302 跳到 `/`。
- 服务端在路由匹配**之前**用一层请求改写处理前缀（原型验证：在整个 Router 外套 `tower::util::MapRequestLayer`，用 `axum::ServiceExt::into_make_service` 启动）。`/p/<id>/x` 改写到现有静态页 `/x`；`/api/p/<id>/x` 改写到现有路由 `/api/x`；`/api/p/<id>/ws` 改写到 `/ws`。现有路由与嵌入式静态文件服务不改；`_next/` 资源是绝对路径，不受前缀影响。
- 改写层解析 `<id>`、取得该 Project 的运行态并放入 Request extensions；处理函数从扩展字段取 Project，不再从全局 state 取。
- 错误：id 不在注册表时先重读一次注册表，仍找不到返回 404 `project_not_found`；路径不存在、`.markplane/` 被删或首次 sync 失败返回 409 `project_unavailable`（附路径），并释放该运行态，下次访问时重试。

### Project 运行态

- 每个 Project 在进程内有一份运行态：`Project`、独立的 WebSocket 广播通道、文件监听器。
- 第一次访问时建立：先执行 `sync_all`，再开监听器（只监视该 Project 的 `.markplane/`），一直保留，不做淘汰；Project 被移出注册表或进程退出时释放。
- 监听器不再监视仓库 `docs/`；文档页是全局的 markplane 自身文档。

### 后台运行

- `serve start` 用 `current_exe()` 重新启动自身（隐藏参数标记子进程），用 `process_group(0)` / `setsid` 脱离终端，stdin 接 `/dev/null`，stdout/stderr 追加写 `serve.log`，工作目录为家目录。不在进程内 fork（多线程进程 fork 后必须 exec），不用 `daemonize` crate（RUSTSEC-2025-0069 无人维护）。父进程不建 tokio 运行时。
- 单实例与存活判断：服务进程（前台、后台一样）对 `serve.pid` 加 `File::try_lock` 并全程持有；端口 bind 成功后才写入 `{pid, port, url, version, started_at}`；退出时清空文件但不删除。其他命令只看“锁是否被持有”，不受 pid 复用影响。
- 端口被占用：服务进程以非零码退出；`start` 的父进程发现子进程提前退出，打印 `serve.log` 末尾并提示 `--port` / `lsof`。
- `stop`：SIGTERM，每 100ms 检查锁是否释放，约 10s 后 SIGKILL；服务端用 graceful shutdown 监听 SIGTERM / SIGINT，并加超时兜底。
- `status`：锁被持有 + pid 文件有内容 + 对端口 TCP 连接成功，即为健康。
- `logs`：调用系统 `tail -n N [-F]`；只在 `start` 时检查，超过约 10MB 改名为 `serve.log.1`，不做运行中轮换。
- 前台 `markplane serve`：保持前台运行；发现锁已被占用时，加锁写入注册表、打开 `https://localhost:{port}/p/{id}/` 后退出 0，不走网络探测，也不需要给注册接口加鉴权。
- 只支持 macOS / Linux。

### HTTPS

- 正式模式只开 HTTPS；`--dev` 保持 HTTP；`--no-tls` 作为逃生口。
- 选 `axum-server` 0.8 的 `tls-rustls` 特性；全程只用 aws-lc-rs 一套加密库，rcgen 关闭默认特性只开 `aws_lc_rs` 与 `pem`，避免同时编进 ring。
- 证书（ECDSA P-256 自签叶子证书）：SAN `localhost`、`127.0.0.1`、`::1`；EKU serverAuth；KeyUsage digitalSignature（critical）；BasicConstraints `CA:FALSE`（critical）；有效期约 800 天（不超过 Apple 对用户信任证书的 825 天上限）；随机序列号，每次轮换换新密钥。不能用 rcgen 的 `generate_simple_self_signed`（默认有效期到 4096 年且不带 EKU，Safari 会拒绝）。
- 存放：`certs/` 目录 0700，`key.pem` 0600，`cert.pem` 0644。证书缺失、剩余不到 30 天或 SAN 不符时重签，重签后提示重新信任。
- 首次生成后打印信任命令：macOS `security add-trusted-cert -r trustRoot -p ssl -k ~/Library/Keychains/login.keychain-db cert.pem`；Linux Chrome `certutil -d sql:$HOME/.local/share/pki/nssdb -A -t "P,," -n markplane-localhost -i cert.pem`；Linux Firefox 对每个 profile 用 `"C,,"`。`update-ca-certificates` 对浏览器无效。
- CORS 白名单、WebSocket Origin 校验与启动时打印的 URL 改为按实际协议（`https://`）生成。前端已按页面协议自动选择 `wss:`。

### 前端

- 当前 Project id 用 `useSyncExternalStore` 读取：服务端快照返回 `null`（与静态预渲染一致），客户端快照从地址栏解析。**不能**在渲染时直接读 `window`：静态导出在构建时预渲染，hydration 后 React 不会修正 `href` 属性，会导致 Cmd-click / 新标签页丢失 Project（原型实测）。原型里验证过的 hook：

```ts
const noopSubscribe = () => () => {};

export function useProjectId(): string | null {
  return useSyncExternalStore(noopSubscribe, currentProjectId, () => null);
}
```

- 所有内部链接与跳转统一走 `ProjectLink` / `useProjectRouter`（约 30 处）；ESLint `no-restricted-imports` 禁止直接引入 `next/link` 与 `useRouter`。
- 当前位置判断（如侧边栏高亮）先剥掉 `/p/<id>` 再比较。
- API 客户端在一处统一加上 `/api/p/<id>/` 前缀；WebSocket 连接带上 Project 前缀（包括开发模式下当前写死的 `NEXT_PUBLIC_WS_URL`）。
- 切换 Project 时重新挂载应用外壳并清空查询缓存。
- `next dev`：rewrites 增加 `/p/:id/:path*` → `/:path*`，API 与 WebSocket 照旧代理到 Rust。
- 删除 `ecosystem.config.cjs` 与 `pm2*` npm scripts；文档同步更新。

## Testing Decisions

- 好的测试只验证外部行为（HTTP 请求与响应、CLI 的输出、退出码与文件效果），不测内部结构。
- **切入点一：进程内的 axum 应用。** 把 `run` 里组装 Router（含前缀改写层）的逻辑抽成工厂函数；测试把注册表放进临时目录，用 `tower::ServiceExt::oneshot` 发请求，不起端口、不走 TLS。覆盖：注册表增删改名与列表、realpath 去重、未初始化目录、`/p/<id>/` 改写、404 `project_not_found` / 409 `project_unavailable`、两个 Project 的数据与 WebSocket 事件互不串扰、文档接口全局、旧地址跳落地页、目录浏览接口。
- **切入点二：CLI 二进制。** 沿用 `crates/markplane-cli/tests/integration.rs` 的 `cargo_bin_cmd!()` + `TempDir` 写法；`XDG_CONFIG_HOME` 指向临时目录、端口随机。覆盖：`serve start/stop/status/logs`、单实例、残留 pid 文件、端口占用报错、后台已有服务时前台 `serve` 只注册并退出、证书自动生成与文件权限、一次 HTTPS 握手成功。
- 前端不新增测试框架；靠 ESLint 规则防止漏改链接。
- 在 CI 里实测一次 `x86_64-unknown-linux-musl` 构建（aws-lc-rs 在 musl 下尚未实测）。

## Out of Scope

- `markplane mcp`（按 Project、stdio 启动，不受影响）。
- 局域网访问与访问鉴权（只服务本机）。
- 自建本地 CA（mkcert 式）证书方案。
- Windows 上的后台运行与证书信任（Windows 只用前台 `markplane serve`）。
- 开机自启（launchd / systemd）。

## Further Notes

- 来源：Map “markplane serve 多项目切换、后台运行与 HTTPS” [[MAP-3fjjb]]。决策细节见：静态导出的前端能否在 /p/<id>/ 路径前缀下运行 [[TASK-rm5w6]]、axum 怎么接 rustls 提供 HTTPS/wss，并用 rcgen 生成浏览器可接受的自签名证书 [[TASK-zaagz]]、markplane 在 macOS/Linux 上把 serve 放到后台运行的做法 [[TASK-fixyf]]、一个 serve 进程如何管理多个 Project 的加载、文件监听与 WebSocket [[TASK-iy25u]]。
- 原始材料（仅本地分支，不合并）：`prototype/project-prefix-routing`、`research/serve-https`、`research/serve-daemon`。
- 参考实现：[bead-me-up-scotty](https://github.com/brendan-appstart/bead-me-up-scotty)。
- 术语见 `GLOSSARY.md`：Project、Project registry。
