---
id: TASK-fixyf
title: markplane 在 macOS/Linux 上把 serve 放到后台运行的做法
status: done
priority: medium
type: research
effort: medium
epic: null
plan: null
map: MAP-3fjjb
depends_on: []
blocks: []
related: []
assignee: chuck
tags: []
position: aU
created: 2026-10-10
updated: 2026-10-10
---

# markplane 在 macOS/Linux 上把 serve 放到后台运行的做法

## Question

`markplane serve start|stop|status|logs` 在 macOS / Linux 上把服务放到后台运行，应采用哪种做法？需要回答：

- 重新启动脱离终端的自身进程（`setsid`、stdin 置空、stdout/stderr 写 `serve.log`）对比 `daemonize` 等 crate 的 fork 方式；与 tokio 运行时的相容性（fork 必须在运行时创建前）。
- pid 文件的写入时机、失效判断（进程已死、pid 被复用），以及端口占用时的处理。
- `stop` 的信号与优雅退出（SIGTERM → 超时 SIGKILL），`status` 如何判断进程健康（pid 存活 + 端口探测 / 健康检查接口）。
- `logs` 的实现（tail / follow）与日志轮换是否需要。
- 前台 `markplane serve` 如何检测已有后台进程并改为“注册当前目录 + 打开浏览器”。

## Answer

调研材料：分支 `research/serve-daemon`（`a28e708`，仅本地），文件 `research/serve-background-daemon.md`（含一手出处）。

推荐做法：

- **后台化**：`serve start` 用 `std::env::current_exe()` 重新启动自身，带隐藏参数 `--daemon-child`；用稳定的 `CommandExt::process_group(0)`（或在 `pre_exec` 里 `setsid`）脱离终端，stdin 接 `/dev/null`，stdout/stderr 追加写 `serve.log`，工作目录设为家目录。
  - 不在进程内 fork：POSIX 与 macOS `fork(2)` 要求多线程进程 fork 后必须 exec。
  - 不用 `daemonize` crate：已被标为无人维护（RUSTSEC-2025-0069）。
  - 父进程不建 tokio 运行时，“fork 必须在运行时之前”的约束自然消失。
- **单实例与 pid 文件**：服务进程（前台、后台都一样）对 `serve.pid` 调 `File::try_lock`（标准库 flock，Rust 1.89 起稳定）并全程持有；端口 bind 成功后才写入 `{pid, port, url, version, started_at}`；其他命令看“锁是否被持有”判断死活，不受 pid 复用影响；退出时只清空文件、不删除。
- **端口占用**：遇到 `AddrInUse` 时服务进程以非零码退出；`start` 的父进程发现子进程提前退出，就打印 `serve.log` 末尾，并提示改用 `--port` 或用 `lsof` 查占用。
- **stop**：先发 SIGTERM，每 100ms 检查锁是否释放，约 10s 后改发 SIGKILL；服务端用 axum `with_graceful_shutdown` 监听 SIGTERM/SIGINT，并加超时兜底，避免卡住。
- **status**：锁被持有 + pid 文件有内容 + 对端口做一次 TCP 连接成功，即视为健康；第一版不需要 HTTPS 客户端。
- **logs**：直接执行系统的 `tail -n N [-F] serve.log`；只在 `start` 时检查，超过约 10MB 就改名为 `serve.log.1`，不做运行中轮换。
- **前台 `serve` 遇到已有服务**：发现锁被占用后，加锁写入 `projects.json`（写临时文件再 rename），打开 `https://localhost:{port}/p/{id}/`，然后退出 0；服务进程遇到不认识的 Project id 时重读注册表。这样不需要网络探测，也不需要给注册接口做鉴权。
