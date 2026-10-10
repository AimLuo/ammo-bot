---
id: TASK-kfzkx
title: 删除 pm2 并更新文档
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-rm3zt
plan: null
depends_on:
- TASK-vnshg
- TASK-2xsvw
- TASK-jn5ka
- TASK-kcivc
blocks: []
related: []
assignee: chuck
tags:
- ready-for-agent
position: ae
created: 2026-10-10
updated: 2026-10-10
---

# 删除 pm2 并更新文档

## Parent

markplane serve：多 Project 切换、后台运行与 HTTPS [[EPIC-rm3zt]]

## What to build

收尾：删除 pm2，并让文档反映新的使用方式。

- 删除 pm2 进程文件与 `pm2:*` npm scripts，以及相关日志忽略规则；前端开发统一用 `npm run dev`（配合 `markplane serve --dev`）。
- 更新 CLI 参考、Web UI 指南、Getting Started、README 与 CLAUDE.md / AGENTS.md 中涉及 `serve` 的部分：Project registry 与落地页、`/p/<id>/` 地址、`serve start/stop/status/logs`、HTTPS 与信任证书、`--dev` / `--no-tls`、用户配置目录中的文件。

## Acceptance criteria

- [ ] 仓库中不再有 pm2 相关文件与脚本
- [ ] 文档中的 `serve` 用法与实际 CLI 帮助一致
- [ ] 按 Getting Started 从零操作可以注册两个 Project、后台运行并通过 HTTPS 访问
