---
id: TASK-hreg4
title: Needs You 标签页
status: planned
priority: medium
type: feature
effort: medium
epic: EPIC-24iwg
plan: null
depends_on:
- TASK-h2vcn
blocks:
- TASK-gtgfz
related: []
assignee: null
tags:
- ready-for-agent
position: az
created: 2026-10-10
updated: 2026-10-10
---

# Needs You 标签页

## Parent

任务卡片评论、Needs You 与会话记录 [[EPIC-24iwg]]

## What to build

`/backlog/` 第三个标签页 "Needs You"：一处看到所有等人处理的 Task。

- `?view=needs-you`，标签旁显示未经筛选的数量；`v` 键在 board → backlog → needs-you 之间循环。
- 收录：未归档、带 `ready-for-human`、状态类别不是 completed / cancelled 的 Task（按类别判断，不写死状态名）。Decision tickets 总是包含，以 Map 标签标出；此标签页隐藏 "Decision tickets" 开关。
- 只读扁平列表：每行优先级、ID、标题、状态徽章、Map / Epic 标签、assignee、最新一条评论预览（`作者 · 日期：首行`，截断）。
- 排序固定为 priority（critical → someday）→ `updated` 倒序 → ID，不显示排序选择器；沿用 priority / epic / tag / assignee 筛选，不加状态筛选。
- 点击行用 `?task=` 打开详情面板；空状态 "Nothing needs you"；实时性依赖现有 WebSocket 查询失效。

## Acceptance criteria

- [ ] 带 `ready-for-human` 的 planned / active Task 出现，completed / cancelled / 已归档的不出现，自定义 workflow 下同样成立
- [ ] Decision tickets 不受开关影响地出现，并显示 Map 标签
- [ ] 排序与筛选符合上述规则；标签上的数量不受筛选影响
- [ ] 行内显示最新评论预览；点击行打开详情面板
- [ ] `v` 键三视图循环；空列表显示 "Nothing needs you"
- [ ] 前端 typecheck、lint、生产构建通过
