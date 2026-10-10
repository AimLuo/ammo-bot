---
id: TASK-2xsvw
title: 在浏览器里添加 Project（目录浏览与 init）
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-rm3zt
plan: null
depends_on:
- TASK-y7nci
blocks:
- TASK-kfzkx
related: []
assignee: chuck
tags:
- ready-for-agent
position: ad
created: 2026-10-10
updated: 2026-10-10
---

# 在浏览器里添加 Project（目录浏览与 init）

## Parent

markplane serve：多 Project 切换、后台运行与 HTTPS [[EPIC-rm3zt]]

## What to build

在浏览器里添加 Project。

- 全局目录浏览接口 `GET /api/fs?path=`：默认从家目录开始，支持 `~`；只列子目录、隐藏点开头的目录；每项标出是否含 `.markplane/`；不限制浏览范围（只服务本机）。
- 添加接口：realpath 去重，重复添加只更新 `lastOpened`；目录不含 `.markplane/` 时返回“未初始化”。
- 全局 init 接口：在选中目录执行与 `markplane init` 相同的初始化，然后注册。
- 目录浏览弹窗：逐级点入、返回上级、直接粘贴路径回车；只有含 `.markplane/` 的目录才能直接“添加”，未初始化目录提供“初始化并添加”。从落地页占位卡与切换器的“添加 Project…”打开。

## Acceptance criteria

- [x] 能从家目录逐级浏览并添加一个已有 Project，添加后跳到 `/p/<id>/`
- [x] 粘贴 `~/...` 路径可直接定位
- [x] 选择未初始化目录可一键初始化并添加，生成的 `.markplane/` 与 CLI `init` 一致
- [x] 重复添加（含符号链接路径）不产生重复条目
- [x] 目录浏览、添加、init 接口有进程内测试覆盖
