---
id: TASK-vnshg
title: 全部内部链接走 ProjectLink，并加 ESLint 规则
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
position: aa
created: 2026-10-10
updated: 2026-10-10
---

# 全部内部链接走 ProjectLink，并加 ESLint 规则

## Parent

markplane serve：多 Project 切换、后台运行与 HTTPS [[EPIC-rm3zt]]

## What to build

前端所有内部链接与跳转都保留当前 Project。提供 `ProjectLink` 与 `useProjectRouter`（基于读取 Project id 的 hook，hydration 后才带上前缀），把剩余的 `<Link>`、`router.push` / `replace` 与命令面板、详情面板、Markdown 内部引用等跳转全部迁移过去；所有“当前位置”判断先剥掉 `/p/<id>`。加 ESLint `no-restricted-imports` 规则：除 `ProjectLink` / `useProjectRouter` 的实现外，禁止直接引入 `next/link` 与 `next/navigation` 的 `useRouter`。

## Acceptance criteria

- [ ] 任意页面上的内部链接，DOM 中的 `href` 都带 `/p/<id>/` 前缀（Cmd-click / 中键 / 复制链接不丢 Project）
- [ ] 带 query 的链接（如 `?map=`、`?node=`）保留前缀与参数
- [ ] `npm run lint` 在直接引入 `next/link` 或 `useRouter` 时失败；当前代码 lint 通过
- [ ] `npm run build` 通过
