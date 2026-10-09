# 07: MAP 页面原型

Type: prototype
Status: resolved
Blocked by: 02, 03, 04
Labels: wayfinder:prototype

## Question

Web UI 中与 notes / plans / roadmap 并列的 maps 页面应长什么样：列表页展示什么；详情页如何呈现 Destination、Decisions so far、frontier、被阻塞的工单（是否用依赖图）、Not yet specified 与 Out of scope？用一个粗糙原型让人反应后定稿。

## Answer

2026-10-09 与用户看原型后确认：

- **详情页选 C（Route graph）**：Decision ticket 按依赖层级从左到右排成路线图，左端 Start、右端 Destination 框（附迷雾区数量提示）；连线为阻塞关系，点选节点时高亮相关连线；节点边框颜色表示状态（decided 半透明、claimed 实线并显示 `@assignee`、frontier、blocked 虚线），节点内显示类型徽标与标题；cancelled（范围外）工单不进图。右侧栏：选中工单的 Question / Answer / Blocked by，其下为 Not yet specified 与 Out of scope；顶部有状态图例。
- **列表页单独做 `/maps`**：卡片显示 ID、状态、标题、进度（已决策 / 总数）与 frontier 数；点进去是路线图详情页（取代原型中的下拉框）。
- **从工单回到 map**：Backlog 默认隐藏决策工单；工单详情显示 `Map` 字段，点击跳到该 map 的路线图并高亮该节点。
- 未选的 A（Document）、B（Board）不保留。

原型（三个版本 + 浮动切换条，假数据）保存在一次性分支 `prototype/map-page`（worktree `.worktrees/prototype-map-page`，commit `ca70651`），未合入；工作区的 UI 源码已恢复为与上游一致。
