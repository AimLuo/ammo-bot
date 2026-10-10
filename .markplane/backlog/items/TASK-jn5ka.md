---
id: TASK-jn5ka
title: 后台运行：serve start/stop/status/logs
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-rm3zt
plan: null
depends_on:
- TASK-pzyhc
blocks:
- TASK-kfzkx
related: []
assignee: chuck
tags:
- ready-for-agent
position: ac
created: 2026-10-10
updated: 2026-10-10
---

# 后台运行：serve start/stop/status/logs

## Parent

markplane serve：多 Project 切换、后台运行与 HTTPS [[EPIC-rm3zt]]

## What to build

由 markplane 自己管理后台服务（只支持 macOS / Linux），做法见 Epic 中“后台运行”。

- `markplane serve start`：重新启动自身（隐藏参数标记子进程），`process_group(0)` / `setsid` 脱离终端，stdin 接 `/dev/null`，输出追加写 `serve.log`，工作目录为家目录；父进程不建 tokio 运行时；子进程提前退出（如端口被占）时打印 `serve.log` 末尾并提示 `--port` / `lsof`。
- 单实例：服务进程（前台、后台一样）对 `serve.pid` 加 `File::try_lock` 并全程持有；端口 bind 成功后写入 `{pid, port, url, version, started_at}`；退出时清空不删除；残留文件靠“锁是否被持有”判断。
- `stop`：SIGTERM，每 100ms 检查锁，约 10s 后 SIGKILL；服务端 graceful shutdown 监听 SIGTERM / SIGINT 并加超时兜底。
- `status`：锁被持有 + pid 文件有内容 + 端口 TCP 连接成功即为运行中，输出 pid、端口、URL、版本。
- `logs`：`tail -n N`，`-f` 跟随；`start` 时日志超过约 10MB 改名为 `serve.log.1`。
- 前台 `markplane serve` 发现已有服务时：加锁注册当前 Project、打开 `<url>/p/<id>/`，退出 0。

## Acceptance criteria

- [ ] `start` 返回终端后服务持续运行；`status` 显示运行中；`stop` 后 `status` 显示未运行
- [ ] 已在运行时再次 `start` 报告已运行而不是起第二个进程
- [ ] 进程被 SIGKILL 后残留的 `serve.pid` 不妨碍下一次 `start`
- [ ] 端口被占用时 `start` 非零退出并输出日志末尾与提示
- [ ] 后台运行时，在另一个 Project 目录运行前台 `serve` 会注册该 Project 并立即退出
- [ ] 以上用 CLI 集成测试覆盖（`XDG_CONFIG_HOME` 指向临时目录、随机端口）
