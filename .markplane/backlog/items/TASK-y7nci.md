---
id: TASK-y7nci
title: 落地页、Project 切换器与 Project 管理
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-rm3zt
plan: null
depends_on:
- TASK-pzyhc
blocks:
- TASK-2xsvw
related: []
assignee: chuck
tags:
- ready-for-agent
position: ab
created: 2026-10-10
updated: 2026-10-10
---

# 落地页、Project 切换器与 Project 管理

## Parent

markplane serve：多 Project 切换、后台运行与 HTTPS [[EPIC-rm3zt]]

## What to build

落地页与 Project 管理。

- `/` 以卡片列出 Project registry：显示名、路径、路径是否有效（失效显示红点），按 `lastOpened` 倒序；注册表为空时显示“添加第一个 Project”占位卡（添加功能在后续票，此处可先占位）。
- 全局注册表接口：列出（附路径是否有效）、移除（不删任何文件，同时释放该 Project 的运行态）、改显示名。
- 侧边栏 Project 切换器：列出已注册 Project 并跳转，含“所有 Project”入口；切换时重新挂载应用外壳并清空查询缓存。
- 前端遇到 404 `project_not_found` / 409 `project_unavailable` 时显示整页提示与“返回 Project 列表”。

## Acceptance criteria

- [ ] `/` 正确列出 Project、排序与失效标记
- [ ] 移除后列表与 `projects.json` 都不再含该 Project，Project 目录文件不变
- [ ] 改名后列表与切换器显示新名称，id 与 URL 不变
- [ ] 从 A 切到 B 后看不到 A 的任何数据
- [ ] 访问未注册 id 或已删除目录的 Project 时显示整页错误并可返回列表
- [ ] 注册表接口有进程内测试覆盖
