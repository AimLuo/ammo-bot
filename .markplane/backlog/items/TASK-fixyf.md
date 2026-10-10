---
id: TASK-fixyf
title: markplane 在 macOS/Linux 上把 serve 放到后台运行的做法
status: draft
priority: medium
type: research
effort: medium
epic: null
plan: null
map: MAP-3fjjb
depends_on: []
blocks: []
related: []
assignee: null
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
