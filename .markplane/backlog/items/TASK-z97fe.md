---
id: TASK-z97fe
title: 评论功能与 Needs You 标签页
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-8aavc
depends_on: []
blocks: []
related: []
assignee: chuck
tags: []
position: al
created: 2026-10-10
updated: 2026-10-10
---

# 评论功能与 Needs You 标签页

## Question

任务卡片的评论与 Needs You 标签页怎么做？

- 评论：新增专用 API（如 `POST /api/tasks/{id}/comments`）还是走现有 PATCH body；作者署名取什么；评论在卡片上还是仅在任务详情面板展示、如何列出历史评论（沿用 `## Comments` / `### {date} {author}` 存储格式）。
- Needs You：作为 `/backlog/` 的第三个视图（`?view=` 参数）列出所有带 `ready-for-human` 的 Task；以列表还是看板呈现；是否包含 Decision tickets。

## Answer

## Answer

### 代码现状（决策依据）

- 评论存在正文末尾 `## Comments` 下，每条 `### {YYYY-MM-DD} {author}`；`Project::add_comment` 持条目文件锁做读-改-写，并更新 `updated`。CLI `markplane comment` 与 MCP `markplane_comment` 的默认作者都是 `$USER` / `$USERNAME` / `me`。
- Web 没有评论 API。`PATCH /api/tasks/{id}` 的 `body` 是整篇替换；详情面板把正文（含评论）当一整篇 Markdown 渲染和编辑。因此人在编辑正文时，agent 刚追加的评论会被覆盖。
- `TaskResponse`（列表和详情都返回）含完整 `body`；文件监听经 WebSocket 让任务查询失效，agent 写的评论会自动出现在界面上。
- `/backlog/` 有 `board`（默认）和 `?view=backlog` 两个视图，`v` 键切换；有 "Decision tickets" 开关（默认隐藏 Map 票）。
- `ready-for-human` 也是分诊标签（"Requires human implementation"），Task 完成后这个标签仍然留着。

### 评论

**存储与结构**

- 沿用现有格式。`## Comments` 段 = 从该标题到下一个 `## ` 标题或文末；段内每个 `### {date} {author}` 开始一条评论，首个空白前是日期，其余是作者（作者可含空格）。第一条评论之前的文字不列出，但保留在文件里。代码围栏内的标题行不参与解析。
- 新评论文本里的 H1–H3 标题行（围栏外）在 core 追加时一律降为 H4。CLI、MCP、Web 三个入口规则一致，保证文件结构始终能解析。
- 日期只到天，不加时间；同一天的多条按文件顺序排列。
- 评论只追加：UI 不提供编辑、删除；需要更正时直接改文件。

**API**

| 项 | 决定 |
| --- | --- |
| 新增评论 | 专用 `POST /api/tasks/{id}/comments`，请求体 `{ "text": "…" }`；服务端调 `add_comment`（带锁），返回 201 和更新后的 `TaskResponse`。纯追加，没有副作用。 |
| 校验 | 去掉首尾空白后为空 → 400；只接受 Task ID。 |
| 读取 | `TaskResponse` 增加 `description`（正文去掉 `## Comments` 段）和 `comments: [{date, author, body}]`（服务端解析）；`body` 仍是完整原文，保持兼容。 |
| 正文编辑 | `PATCH` 增加 `description` 字段：服务端在锁内用它替换非评论部分，并把磁盘上当前的 `## Comments` 段原样接回文末。UI 改用 `description`；`body` 整篇替换的语义保留给其他调用方。 |

**作者署名**

- 作者由服务端决定，客户端不能传，避免冒充 `dispatcher`。
- `default_user_name()` 先读环境变量 `MARKPLANE_AUTHOR`，再依次回退到 `$USER` / `$USERNAME` / `me`；CLI、MCP、serve 共用这一逻辑。
- 三类作者：

| 写入方 | 作者 |
| --- | --- |
| 人（Web 或 CLI） | serve / CLI 进程的 `$USER`，同一个人在 Web 和 CLI 里同名 |
| Dispatcher 自己写的评论（失败原因、合并结果） | `dispatcher`（显式传入，与 assignee 一致） |
| Stage 中的 agent 会话 | `agent-<stage>`，即 `agent-implement` / `agent-fix` / `agent-merge`。Dispatcher 启动子进程时设置 `MARKPLANE_AUTHOR` |

- UI 给 `dispatcher` 和 `agent-*` 作者加机器人图标，与人的评论区分开。

**界面**

- 详情面板（TaskDetailSheet）：元数据 → 描述（`description`，编辑器也只编辑这一部分）→ "Comments (n)" 段。评论按时间正序排列，每条显示作者、日期和 Markdown 渲染的正文（支持 wiki-link），不折叠、不分页；最底部是评论输入框。
- 看板卡片（TaskCard）：有评论时显示评论图标和数量，卡片上不放输入框。
- 已归档的 Task 只能看评论，不显示输入框。
- 范围只到 Task（含 Decision tickets）；Epic / Plan / Note / Map 的 Web 评论不做。

**可复用的评论框 `CommentComposer`**

- 用现有 `ui/textarea` 输入纯 Markdown，不做富文本。
- 主按钮"评论"；Cmd/Ctrl+Enter 提交；内容为空或请求进行中时禁用。
- 成功后清空；失败时保留文字并弹出 toast；关闭面板不保留草稿。
- Props：`onSubmit(text)`、`submitLabel`、`extraActions`（附加按钮插槽），以及 `disabled`。comment and continue（[[TASK-pd8bj]]）可以往插槽里加"评论并继续"；它的语义和接口由该票决定。

### Needs You 标签页

| 项 | 决定 |
| --- | --- |
| 位置 | `/backlog/` 第三个标签，`?view=needs-you`，标签名 "Needs You"，旁边显示未经筛选的数量；`v` 键在 board → backlog → needs-you 之间循环。 |
| 收录 | 未归档、带 `ready-for-human`、状态类别不是 completed / cancelled 的全部 Task（draft、backlog、planned、active 都算）。排除完成态的原因：分诊标签在 Task 完成后仍然留着，不排除的话这个视图会越积越多。需要相应修订术语表。 |
| Decision tickets | 总是包含，用 Map 标签区分；这个标签页里隐藏 "Decision tickets" 开关。 |
| 呈现 | 只读的扁平列表，不拖拽、不分组、不做看板。每行显示优先级、ID、标题、状态徽章、Map / Epic 标签、assignee，以及最新一条评论的预览（`作者 · 日期：首行`，截断）。Dispatcher 写的失败原因可以直接在这里看到。 |
| 排序 | 固定为 priority（critical → someday）→ `updated` 倒序 → id；不显示排序选择器。 |
| 筛选 | 沿用 priority / epic / tag / assignee 筛选；不加状态筛选。 |
| 交互 | 点击一行打开同一个详情面板（`?task=`），在面板里看评论、写评论、改标签。这里不提供"处理完"之类的快捷按钮：移除 `ready-for-human` 用现有的标签编辑器；失败 Run 的后续处理归 [[TASK-pd8bj]] 和 [[TASK-k6d9y]]。 |
| 空状态 | 显示 "Nothing needs you"。 |
| 实时性 | 依赖现有 WebSocket 刷新查询，不另加轮询。 |

### 对并行票的假设

- [[TASK-2vpdd]]：评论始终写在 ticket 文件里，不放进 Run 记录。Run 记录放哪里不影响本方案。
- [[TASK-cq846]]：评论和 Needs You 都不需要设置项。
- [[TASK-by9nc]]："查看会话记录"入口放在哪里、长什么样不在这里决定；详情面板评论段的上方或旁边可以留出位置。

### 自问自答清单（均采纳推荐答案）

1. 评论走专用 POST 还是 PATCH `body`？→ 专用 `POST /api/tasks/{id}/comments`，服务端带锁追加，避免整篇覆盖丢掉 agent 评论。
2. 描述编辑器还编辑评论吗？→ 不编辑。API 分出 `description` / `comments`，PATCH `description` 时服务端保留磁盘上的评论段。
3. 评论能编辑、删除吗？→ 不能，只追加。
4. 作者取什么，客户端能指定吗？→ 服务端决定：人用 `$USER`，Dispatcher 用 `dispatcher`，agent 用 `agent-<stage>`（通过 `MARKPLANE_AUTHOR`）；客户端不能传。
5. 评论正文里的标题会破坏结构，怎么办？→ core 追加时把 H1–H3 降为 H4；解析时忽略围栏内的内容。
6. 要不要在日期后加时间？→ 不加，保持现有格式，按文件顺序排列。
7. 评论显示在哪里？→ 详情面板里列出完整历史并放输入框；卡片只显示数量。
8. 输入框做成富文本吗？→ 不做，纯 Markdown textarea，留附加按钮插槽供 comment and continue 复用。
9. Epic / Plan / Note 也做 Web 评论吗？→ 不做，只做 Task。
10. 已归档 Task 能评论吗？→ 不能，只读。
11. Needs You 放在哪里？→ `/backlog/?view=needs-you` 第三个标签。
12. 已完成、已取消的 Task 算不算？→ 不算，并修订术语表。
13. 包含 Decision tickets 吗？→ 包含，不受开关影响。
14. 用列表还是看板？→ 只读扁平列表，带最新评论预览。
15. 怎么排序、怎么筛选？→ priority → updated 倒序；沿用四个筛选，不加状态筛选，不显示排序选择器。
16. 要不要快捷的"处理完"按钮？→ 不要，用标签编辑器；失败 Run 的后续由 pd8bj 决定。
17. 要不要全局角标或跨 Project 汇总？→ 不要；只在标签上显示数量，范围限于当前 Project。
18. 怎么拿到实时更新？→ 复用现有 WebSocket 刷新。
