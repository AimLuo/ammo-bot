---
id: EPIC-24iwg
title: 任务卡片评论、Needs You 与会话记录
status: now
priority: medium
started: null
target: null
related:
- EPIC-ce5ps
tags:
- ready-for-agent
created: 2026-10-10
updated: 2026-10-10
---

# 任务卡片评论、Needs You 与会话记录

## Problem Statement

在 Web 界面里没法给任务写评论：评论只能用 CLI 或 MCP 追加，而详情面板把正文连同 `## Comments` 段当成一整篇 Markdown 来编辑。人保存描述时，会把 agent 刚追加的评论一起覆盖掉。

Dispatcher 上线后，agent 会无人值守地实现 Implementation ticket。人需要在界面上看到 agent 和 Dispatcher 写的评论（review 发现、失败原因、合并结果）并回复；需要一处看到所有等自己处理的任务；还需要查看某张 ticket 的 agent 会话到底做了什么。这三件事现在都做不到。

## Solution

- **评论**：详情面板把描述和评论分开。描述编辑器只编辑描述；评论按时间正序列出，底部是评论框。评论走专用接口追加，作者由服务端决定，保存描述不会再覆盖评论。看板卡片显示评论数。
- **Needs You**：`/backlog/` 新增第三个标签页，列出所有带 `ready-for-human`、尚未完成或取消的 Task，每行带最新一条评论的预览。
- **会话记录**：详情面板新增最小的 Run 段和 View sessions 按钮，打开会话记录弹窗。左侧按 Run → Stage → 会话 → 调用列出，右侧显示所选会话的完整记录（读本机 agent 的会话文件），手动刷新。
- 本 Spec 定义 Run record 的文件格式，并提供只读接口；写 Run record 的是后续的 Agent Dispatcher Spec。

## User Stories

### 评论

1. As a developer, I want to write a Comment on a Task from its detail panel, so that I can respond to an agent or record a decision without leaving the browser.
2. As a developer, I want my Comment appended rather than merged into the description, so that posting it never overwrites anything else in the file.
3. As a developer, I want editing a Task's description to leave existing Comments untouched, so that an agent's Comment written while I was editing is not lost.
4. As a developer, I want Comments listed oldest-first under a "Comments (n)" heading below the description, so that I can read a conversation in order.
5. As a developer, I want each Comment to show its author, date and Markdown-rendered body with working wiki-links, so that Comments read like the rest of the item.
6. As a developer, I want Comments written by the Dispatcher or an agent to carry a robot icon, so that I can tell machine-written Comments from people's at a glance.
7. As a developer, I want Cmd/Ctrl+Enter to submit a Comment, so that I can write quickly from the keyboard.
8. As a developer, I want the Comment button disabled while the box is empty or a request is in flight, so that I can't post empty or duplicate Comments.
9. As a developer, I want my draft kept in the box with an error toast if posting fails, so that I don't lose what I typed.
10. As a developer, I want the box cleared after a successful post, so that it's ready for the next Comment.
11. As a developer, I want the server, not my browser, to decide a Comment's author, so that nobody can post as `dispatcher` or an agent.
12. As a developer, I want my Web Comments signed with the same name my CLI Comments use, so that one person appears under one name.
13. As a developer, I want archived Tasks to show their Comments read-only, so that I can review history without adding to it.
14. As a developer, I want headings inside a Comment's text demoted so they can't break the Comments section, so that the file stays parseable whichever entry point wrote it.
15. As a developer, I want a Comment count on the board card when a Task has Comments, so that I can spot discussion without opening the card.
16. As a developer, I want new Comments from agents to appear in the open panel automatically, so that I don't need to reload.
17. As an agent run by the Dispatcher, I want my `markplane comment` calls signed with my Stage name automatically, so that I never have to pass an author.
18. As the Dispatcher, I want to sign my own Comments as `dispatcher`, so that failure reasons and merge results are attributed consistently with the assignee.

### Needs You

19. As a developer, I want a "Needs You" tab on the backlog page, so that I have one place for everything waiting on me.
20. As a developer, I want Needs You to include every unarchived Task tagged `ready-for-human` that is not completed or cancelled, whatever its other status, so that finished Tasks don't pile up there.
21. As a developer, I want Decision tickets always included in Needs You, marked by their Map label, so that wayfinding questions waiting on me aren't hidden by the Decision tickets toggle.
22. As a developer, I want each row to show priority, ID, title, status, Map/Epic label, assignee and a one-line preview of the newest Comment, so that I can see why a Task needs me without opening it.
23. As a developer, I want Needs You sorted by priority, then most recently updated, then ID, so that the most urgent and freshest items are on top.
24. As a developer, I want the existing priority / epic / tag / assignee filters on Needs You, so that I can narrow a long list.
25. As a developer, I want clicking a row to open the same detail panel, so that I can read and reply in the usual place.
26. As a developer, I want the tab to show its unfiltered count, so that I know how much is waiting before switching.
27. As a developer, I want `v` to cycle board → backlog → needs-you, so that keyboard navigation covers the new tab.
28. As a developer, I want "Nothing needs you" when the list is empty, so that an empty list reads as good news rather than a broken page.

### Run 段与会话记录

29. As a developer, I want a Run section in the detail panel when this machine has a Run record for the Task, so that I know agents have worked on it.
30. As a developer, I want a grey "No local Run record — it ran on another machine" line when the Task is assigned to `dispatcher` but this machine has no record, so that I understand why there's nothing to show.
31. As a developer, I want a "View sessions" button in the Run section header, so that I can open the session log from where the Run is shown.
32. As a developer, I want the session log to open as a large centered dialog with a list on the left and the selected session on the right, so that I can browse many sessions without losing my place.
33. As a developer, I want the newest Run first, Stages in pipeline order (implement → fix → merge), and sessions grouped by session ID with their invocations underneath, so that I can follow what happened.
34. As a developer, I want each invocation to show how it was triggered (initial, resume, continue) and to flag a resume that silently started a new session, so that I can understand gaps in a session.
35. As a developer, I want the right pane to show the whole session with dividers marking each invocation, and clicking an invocation to scroll to it, so that earlier turns stay visible as context.
36. As a developer, I want toggles for Dispatcher prompts, agent text and tool calls, so that I can focus on what the agent said or did.
37. As a developer, I want tool calls folded into one expandable line per turn, so that long tool sequences don't drown the conversation.
38. As a developer, I want only the last 60 messages shown, with "show earlier" and "show all", so that very long sessions open quickly.
39. As a developer, I want input/output token counts per invocation and totals per session and Run, with cache detail in a tooltip, so that I can see what a Run cost in tokens.
40. As a developer, I want a manual Refresh button in the dialog header, so that I can load newer messages of a running session on demand.
41. As a developer, I want a session whose transcript file is missing or unreadable to stay in the list with an "unavailable" notice, so that one bad file doesn't hide the others.
42. As a developer, I want the session log to work for archived Tasks and for Runs whose worktree was removed, so that history survives cleanup.
43. As a developer, I want a clear empty state in the dialog when this machine has no Run record, so that the button never leads to a blank screen.

## Implementation Decisions

### 评论模型（core）

- 存储格式不变：`## Comments` 段从该标题到下一个 `## ` 标题或文末；段内每个 `### {YYYY-MM-DD} {author}` 开始一条评论，首个空白前是日期，其余是作者（可含空格）。第一条评论之前的文字保留但不列出；代码围栏内的标题行不参与解析。
- 追加时，新评论文本里围栏外的 H1–H3 标题行一律降为 H4。CLI、MCP、Web 三个入口规则一致。
- 日期只到天，同一天按文件顺序。评论只追加，界面不提供编辑或删除；要更正就直接改文件。
- 默认作者的取值规则由 CLI、MCP、serve 共用：先读环境变量 `MARKPLANE_AUTHOR`，再依次回退到 `$USER`、`$USERNAME`、`me`。
- 新增解析：把条目正文拆成 `description`（去掉评论段的正文）和评论列表 `{date, author, body}`。新增"只替换描述"的写入：在条目锁内替换非评论部分，并把磁盘上当前的 `## Comments` 段原样接回文末。

### Web API

- 新增 `POST /api/tasks/{id}/comments`，请求体 `{ "text": "…" }`：去掉首尾空白后为空返回 400；只接受 Task ID；在条目锁内追加；返回 201 和更新后的 Task。纯追加，没有其他副作用。作者取 serve 进程的默认作者，客户端不能指定。
- Task 响应增加 `description` 和 `comments: [{date, author, body}]`；`body` 仍是完整原文，保持兼容。
- `PATCH /api/tasks/{id}` 增加 `description` 字段：只替换非评论部分，在锁内把磁盘上的评论段接回去。`body` 整篇替换的语义保留给其他调用方；界面改用 `description`。
- 评论变化沿用现有的文件变化 WebSocket 事件和查询失效到达界面。

### Needs You

- `/backlog/` 第三个视图 `?view=needs-you`，标签名 "Needs You"，旁边显示未经筛选的数量；`v` 键在 board → backlog → needs-you 之间循环。
- 收录：未归档、带 `ready-for-human`、状态类别不是 completed / cancelled 的 Task（按类别判断，不写死状态名）。Decision tickets 总是包含；这个标签页里隐藏 "Decision tickets" 开关。
- 只读扁平列表：不拖拽、不分组、不做看板。每行：优先级、ID、标题、状态徽章、Map / Epic 标签、assignee、最新一条评论预览（`作者 · 日期：首行`，截断）。
- 排序固定为 priority（critical → someday）→ `updated` 倒序 → ID，不显示排序选择器。沿用 priority / epic / tag / assignee 筛选，不加状态筛选。
- 点击行用 `?task=` 打开详情面板。不提供"处理完"快捷按钮：移除 `ready-for-human` 用现有的标签编辑器。空状态 "Nothing needs you"。实时性依赖现有 WebSocket 查询失效。
- 术语表里 Needs You 的定义已经改为排除完成和取消的 Task。

### 详情面板与卡片

- 详情面板顺序：元数据 → Run 段（有时才显示）→ 描述（编辑器只编辑这一部分）→ "Comments (n)" → 底部评论框。评论不折叠、不分页。已归档的 Task 评论只读，不显示评论框。
- 可复用的 `CommentComposer`：用现有 textarea 组件输入纯 Markdown；主按钮 "Comment"；Cmd/Ctrl+Enter 提交；内容为空或请求进行中时禁用；成功后清空；失败时保留文字并弹 toast；不保存草稿。Props：`onSubmit(text)`、`submitLabel`、`extraActions`（附加按钮插槽，Agent Dispatcher Spec 会放 "Comment & continue"）、`disabled`。
- 卡片在有评论时显示评论图标和数量，卡片上不放评论框。
- 作者是 `dispatcher` 或 `agent-*` 的评论加机器人图标。
- 界面文案跟随看板，用英文。

### Run record 格式（本 Spec 定义，Agent Dispatcher Spec 写入）

- 每张 Implementation ticket 一个 JSON 文件 `.markplane/.runs/<TASK-id>.json`，不进 git：目录里自带内容为 `*` 的 `.gitignore`，做法同现有的条目锁目录。只存本机，不跨机器共享。只有 Dispatcher 写，临时文件加 rename 原子写入。文件监听只对 `.md` 和 `config.yaml` 推事件，不会因它触发。
- 文件里是 `runs` 数组，每次 Run 追加一项。Run 字段：序号（从 1 起）、状态（`running` / `merge-queued` / `succeeded` / `failed` / `stopped`）、当前 Stage、worktree 绝对路径、分支、基点 SHA、merge 目标分支、合并提交 SHA、是否已推送、开始与结束时间、失败原因（对应 Dispatcher 的失败判定之一），以及清理字段 `worktree_removed_at`、`branch_removed_at`、`branch_head_at_removal`、`cleanup_blocked`、`cleanup_trigger`。
- 每个 Run 有 `attempts` 数组，每启动一次 agent 进程追加一条：Stage、模型快照、超时快照、会话（chat）ID、agent 工作目录、可选的已解析 transcript 路径、PID、进程组 ID、agent config dir、触发方式（`initial` / `resume` / `continue`）、开始与结束时间、退出码、末行判定、结果（`ok` / `failed` / `timeout` / `interrupted` / `resume_mismatch`）、`usage`（`inputTokens` / `outputTokens` / `cacheReadTokens` / `cacheWriteTokens`，未知时为 null）、`duration_api_ms`、stderr 末尾片段。
- 记录里不存 ticket 的状态、标签、assignee；两者不一致时以 ticket 为准。
- 记录永久保留，不随清理、归档或 unarchive 删除。

### Run 与会话接口（只读）

- `GET /api/runs/{task_id}`（Project 前缀下）：返回完整 Run record；本机没有记录时返回空数组。
- 会话记录接口（Project 前缀下，按 ticket ID 和会话 ID 定位）读取 agent 的会话文件 `${CURSOR_DATA_DIR:-~/.cursor}/projects/<slug>/agent-transcripts/<id>/<id>.jsonl`。slug 由该次调用的工作目录正向计算：所有非 `[A-Za-z0-9]` 字符换成 `-`，合并连续的 `-`，去掉首尾 `-`。这个映射有损，必须由路径正向算；记录里有已解析的 transcript 路径时优先用它。
- 每行格式为 `{"role":"user"|"assistant","message":{"content":[text|tool_use{name,input}]}}`，末行可能是 `{"type":"turn_ended","status":…}`；文件不含工具结果和 thinking。文件是增量重建的快照，每次请求整文件重读。文件不存在或读不了时返回明确的"不可用"结果，而不是错误页。
- `-p` 会话没有标题，弹窗用 Stage 和触发方式标注会话。
- 弹窗不响应 WebSocket 的 Run 事件，只在点 Refresh 时重新读取。

### 最小 Run 段与会话记录弹窗（原型：Variant A）

- Run 段：本机有 Run record 时出现在元数据和描述之间；assignee 是 `dispatcher` 但本机没有记录时，只显示一行灰字 "No local Run record — it ran on another machine" 和 View sessions 按钮。头部右侧是操作位，本 Spec 只放 "View sessions"；RunBadge、Stage 进度、Details 和其他操作由 Agent Dispatcher Spec 补上。
- 弹窗按原型分支 `prototype/session-modal` 选定的 Variant A 实现：
  - 大尺寸居中弹窗；左侧最新的 Run 在最前 → Stage 按流程顺序 → 按会话 ID 分组 → 各次调用，标出触发方式和 `resume_mismatch`。
  - 右侧显示所选会话的完整记录，用分隔线标出每次调用；点左侧的调用时滚到对应位置。
  - Dispatcher prompt、agent 文本、工具调用三类可分别开关；工具调用每轮折成一行，可展开看输入。
  - 显示最后 60 条消息，顶部 "show earlier" / "show all"。
  - Refresh 按钮在头部。
  - 用量：每次调用显示输入 / 输出 token 文本，会话和 Run 显示合计，有调用用量未知时合计前加 `≥`；缓存明细放悬停提示。
  - 空状态："本机没有 Run 记录"、"会话记录不可用"。已归档 ticket 和 worktree 已删的 Run 同样可用，只依赖记录里的会话指针。

## Testing Decisions

- 好的测试只验证外部行为：经最高层接缝发请求，检查返回和磁盘上的文件，不测内部函数。
- **服务端 API（主接缝）**：在进程内构建 app、对临时 Project 发一次性 JSON 请求，沿用 serve 命令里已有的测试写法（Project 前缀、注册表、config 等测试是先例）。覆盖：
  - 发评论会以服务端决定的作者追加，并返回解析后的评论列表；空文本返回 400；非 Task ID 被拒绝。
  - 客户端读取之后、文件里又被追加了评论，此时 PATCH `description` 仍保留那条评论。
  - 追加时标题降级；解析时忽略围栏内的标题。
  - `GET /api/runs/{task_id}` 有 fixture 记录和没有记录两种情况。
  - 会话记录接口：在临时 `CURSOR_DATA_DIR` 下放 fixture 会话文件（slug 由 worktree 形式的路径算出），以及文件缺失、末行被截断的情况。
- **core 单元测试**：评论段解析与描述拆分、作者取值顺序（`MARKPLANE_AUTHOR` 优先），放在现有 core 测试旁边。
- **CLI 集成测试**：`markplane comment` 遵守 `MARKPLANE_AUTHOR`，沿用现有调用二进制的集成测试写法。
- **前端**：不引入测试框架；改动的界面代码必须通过 typecheck、lint 和生产构建。

## Out of Scope

- 富文本评论；界面编辑或删除评论。
- 在 Epic / Plan / Note / Map 上从界面评论。
- 跨 Project 汇总的 Needs You，以及全局侧栏上的 Needs You 角标。
- 会话记录实时刷新（只做手动 Refresh）。
- 把会话内容复制进项目，或跨机器共享 Run record。
- 写 Run record、卡片上的 RunBadge、Stage 进度、Resume / Continue / Clean up 操作：都属于 Agent Dispatcher Spec。

## Further Notes

- 来自 Map「Agent Dispatcher 与任务卡片增强」[[MAP-8aavc]]。依据的决策：评论功能与 Needs You 标签页 [[TASK-z97fe]]、会话记录弹窗 [[TASK-by9nc]]、Run 记录存放位置 [[TASK-2vpdd]]、agent CLI 无头调用契约 [[TASK-g73u8]]；另有来自无人值守的权限与成本边界 [[TASK-kbyh2]]、重启恢复与 comment and continue [[TASK-pd8bj]]、遗留 worktree 与分支的清理 [[TASK-6hqkh]] 的约束。
- 本 Spec 先于 Agent Dispatcher Spec 实现；后者依赖这里的评论框插槽、Run record 格式和 Run 段。
- 会话记录弹窗的原型在分支 `prototype/session-modal`（选定 Variant A）。实现时重写，不要直接搬原型代码。
