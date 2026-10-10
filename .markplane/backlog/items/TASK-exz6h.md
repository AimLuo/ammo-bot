---
id: TASK-exz6h
title: Run 在界面上的呈现
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
position: ao
created: 2026-10-10
updated: 2026-10-10
---

# Run 在界面上的呈现

## Question

Run 在界面上怎么呈现？

- 任务卡片上的运行中标记：显示什么（当前 Stage、排队等 merge、已用时长），点开去哪。
- 是否需要 Dispatcher 总览面板（跨 Project 列出进行中 / 排队的 Run），放在哪；全局并发名额的占用怎么展示。
- 实时性：文件监听不会因 `.markplane/.runs/*.json` 写入而推送事件（见 [[TASK-2vpdd]]），Run 状态变化需要 Dispatcher 主动经 WebSocket 推给界面；推哪些事件、粒度多细。
- 与已定入口的关系：扫描开关在看板顶部、设置在 Settings 的 Dispatcher 标签页（见 [[TASK-cq846]]）；"恢复执行"按钮的位置由 [[TASK-pd8bj]] 决定，本票只留位置。

## Answer

## Answer

### 代码现状（决策依据）

- WebSocket 按 Project 分通道：`/api/p/<id>/ws`，每个 Project 运行时一个 `broadcast` 通道；着陆页 `/` 没有 WebSocket。现有事件都是"薄事件"（`file_changed` / `config_changed` / `sync_complete` / `doc_changed`），界面收到后只让对应查询失效、重新拉取。
- 文件监听只对 `.md` 和 `config.yaml` 推事件，`.markplane/.runs/*.json` 的写入不会推（[[TASK-2vpdd]]）。
- 界面重连时（`onopen`）不重新拉取，断线期间的事件会丢；广播积压（lagged）会让连接断开再重连。QueryClient 的 `staleTime` 是 30 秒，`refetchOnWindowFocus` 关闭。
- 看板列是 planned / active / completed 三个类别，Run 中的 ticket 在 `in-progress` 列；卡片可以拖动改状态，底部一行显示 `@assignee`。
- 详情面板（TaskDetailSheet）：元数据 → 正文；状态和 assignee 都能直接改。
- 已有全局（不带 Project 前缀）API 的机制（`GLOBAL_API_PREFIXES`，如 `/api/projects`）。
- assignee 在 Run 成功或失败后仍保留 `dispatcher`（[[TASK-k6d9y]]），所以不能用 assignee 判断"正在跑"。

### 结论

#### 1. 数据来源与 API

界面上的 Run 状态**只来自 Dispatcher**（内存里的活动 Run 加上 `.runs/` 文件），不写进 `TaskResponse`，因为 ticket 和 Run 记录分属两类事实（[[TASK-2vpdd]]）。

| 端点 | 范围 | 返回 | 用途 |
| --- | --- | --- | --- |
| `GET /api/runs` | 按 Project | 本 Project 每个有 Run 记录的 ticket 的**最新一次** Run 摘要：`task_id`、Run 序号、显示状态、当前 Stage、Stage 开始时间、Run 开始时间、merge 排队位次、失败原因、结束时间 | 卡片、列表行上的 Run 标记 |
| `GET /api/runs/{task_id}` | 按 Project | 该 ticket 的完整 Run 记录（全部 Run 和每次调用）；没有记录时返回空数组 | 详情面板的 Run 段；会话记录弹窗（[[TASK-by9nc]]）也用它 |
| `GET /api/dispatcher` | 全局（与 `/api/projects` 同级） | 并发上限、已占名额（含排队等 merge 的）、运行中数和排队数；按 Project 分组的活动 Run 列表（ticket id、当前标题、显示状态、Stage、Stage 开始时间、排队位次）；每个 Project 的扫描开关状态（只读） | 名额徽章与总览弹出层、Settings 页 |

**显示状态**由服务端统一算出，客户端不自己拼：

| 显示状态 | 判定 |
| --- | --- |
| `running`（附 Stage：implement / fix / merge） | 内存中有该 Run，且正在跑某个 Stage |
| `merge-queued` | 内存中有该 Run，在等本 Project 的 merge 位 |
| `interrupted` | 记录是非终态（`running` / `merge-queued`），但本 serve 内存里没有这个 Run（serve 重启过） |
| `failed` / `stopped` / `succeeded` | 记录的终态 |

客户端只额外派生一个状态 **stopping**：Run 仍是 `running` / `merge-queued`，但 ticket 已不在 active 类别，或 assignee 已不是 `dispatcher`。这时 Dispatcher 会在当前 Stage 结束时静默停止（[[TASK-k6d9y]]）。

#### 2. 任务卡片上的 Run 标记（`RunBadge`）

- **位置**：卡片底部 `@assignee` 那一行的右侧；没有 assignee 时单独占一行。
- **按状态显示**（界面文案跟随看板，用英文）：

| 状态 | 文案 | 视觉 |
| --- | --- | --- |
| running | `implement · 12m` / `fix · 3m` / `merge · 1m` | `--status-in-progress` 色、旋转图标；卡片加同色细边框 |
| merge-queued | `merge queue #2 · 5m` | 时钟图标，同色 |
| stopping | `Stopping · implement` | 灰色 |
| interrupted | `Interrupted · fix` | 琥珀色、暂停图标，不显示时长 |
| failed | `Run failed · merge` | destructive 色、警告图标 |
| stopped | `Run stopped` | 灰色；只在 ticket 仍处于 active 类别时显示 |
| succeeded | 不显示 | ticket 已是 done，状态本身足够 |

- `failed` / `interrupted` 只在 ticket 仍处于 active 类别时显示；人已经把 ticket 改走（如改回 `planned`、`done`、`cancelled`）就不再显示，以 ticket 为准。
- **时长**：显示**当前 Stage 已用时长**（直接对应每个 Stage 的超时），按分钟取整，不足 1 分钟显示 `<1m`；全页共用一个 30 秒刷新的计时器，由客户端根据开始时间计算。悬停提示里给出：Run #n、Run 总用时、Stage 开始时间、本 Stage 模型与超时、本 Stage 调用次数。
- 动画遵守 `prefers-reduced-motion`（关闭动画时显示静态图标）。
- **点开去哪**：标记本身不单独可点（避免和拖动冲突）。点卡片照旧打开同一个详情面板（`?task=`），在那里看 Run 段。
- **其他位置复用同一个 `RunBadge`**：Needs You 列表行（[[TASK-z97fe]]）、Dashboard 的 Active Work 行。Map 页、Roadmap、侧栏、Project 切换器不显示。

#### 3. 详情面板的 Run 段

- **位置**：元数据和正文之间，标题 "Run"。只在本机有 Run 记录时出现。如果 assignee 是 `dispatcher` 但本机没有记录，只显示一行灰字 "No local Run record — it ran on another machine"。
- **内容**（最新一次 Run）：
  - 头部：`RunBadge`、`Run #n`、开始时间、总用时；右侧是**操作位**（见第 6 节）。
  - Stage 进度：`implement → fix → merge` 横排，只有 `REVIEW: issues` 时才出现 fix。每步显示结果图标（进行中 / 成功 / 失败 / 跳过）、实际模型、用时；排队时 merge 一步显示 "queued #n"。
  - 失败原因：完整显示（对应六条失败判定之一）。
  - "Details" 折叠区：worktree 路径（带复制按钮）、分支、基点 SHA、merge 目标分支、合并 SHA、是否已推送。
  - "Earlier runs (n)" 折叠区：之前每次 Run 一行（序号、终态、结束 Stage、日期）。
- 会话 ID 和会话内容不在这里展开，归会话记录弹窗（[[TASK-by9nc]]）。

#### 4. 全局名额与 Dispatcher 总览

- **不做单独页面**，也不改着陆页。总览做成看板顶部的**名额徽章**和它的弹出层。
- **徽章位置**：看板顶部，紧挨扫描开关（与 "Decision tickets" 同一行，[[TASK-cq846]]）。文案 `Slots 12/20`，始终显示；占满时换成警示色。数字包含排队等 merge 的 Run（[[TASK-k6d9y]]）。
- **点开徽章弹出总览**：
  - 顶部一行：`running 9 · merge queue 3 · limit 20`。
  - 按 Project 分组，当前 Project 排第一，其余按名称排序；组头显示 Project 名和该 Project 扫描开关的开 / 关（只读）。
  - 每行：ticket id、标题、`RunBadge`。点击跳到 `/p/<id>/backlog?task=<TASK-id>`，跨 Project 时照常切换。
  - 没有活动 Run 时显示 "No active runs"。
  - 只列活动 Run（running / merge-queued）。interrupted 的 Run 不占名额，不列在这里，留在各自卡片上。
- **Settings → Dispatcher 标签页**的"本机 serve 全局"卡片，在并发上限输入框旁显示 `In use 12`，数据来自同一个 `/api/dispatcher`。

#### 5. 实时推送

沿用"薄事件 + 查询失效"的模式，由 Dispatcher 主动经 WebSocket 推送：

| 事件 | 发到 | 负载 | 何时发 | 界面动作 |
| --- | --- | --- | --- | --- |
| `run_changed` | 该 Run 所属 Project 的通道 | `task_id`、Run 序号、显示状态、Stage | 每次写 Run 记录之后都发：拾取、调用开始、拿到会话 ID、调用结束、进入或离开 merge 队列、成功、失败、静默停止 | 让 `["runs"]` 和 `["runs", task_id]` 失效 |
| `dispatcher_changed` | **所有**已建立运行时的 Project 通道 | 无 | 名额占用变化、任一 Run 的显示状态或 Stage 变化、排队位次变化、并发上限修改、任一 Project 扫描开关切换 | 让 `["dispatcher"]` 失效 |

- 不推进度心跳和 agent 输出流；时长由客户端算，会话内容只在弹窗里手动刷新（地图已把实时刷新划出范围）。
- ticket 文件本身的变化（状态、标签、评论）照旧走 `file_changed`，不重复推。
- **重连补齐**：WebSocket 重连成功（不是首次连接）时，让全部查询失效。这同时补上了现有"断线期间事件丢失"的缺口。
- **失败提示**：当前打开的 Project 收到 `run_changed` 且状态是 `failed` 时，弹出 toast `TASK-xxx run failed at <stage>`，附 "Open" 按钮打开详情面板。成功、停止不弹。不做浏览器桌面通知。

#### 6. 与其他入口的关系（只留位置）

- **"恢复执行"**：留两个位置，由 [[TASK-pd8bj]] 选用其一或两者都用：(a) 详情面板 Run 段头部的操作位，按 Run 恢复；(b) 看板顶部名额徽章旁，全局或按 Project 恢复。interrupted 状态的卡片标记就是需要恢复的提示。
- **"查看会话记录"**：建议入口放在 Run 段头部操作位，最终由 [[TASK-by9nc]] 决定。
- **comment and continue**：仍走评论框的附加按钮位（[[TASK-z97fe]]），本票不另设入口。
- **遗留 worktree 清理**：如果 [[TASK-6hqkh]] 需要手动入口，也放在 Run 段操作位。
- **不提供**界面上的"终止 Run"（强杀 agent 进程），与 [[TASK-k6d9y]] "不中途杀进程"一致。

#### 7. 人在运行中改 ticket 的提示

ticket 有活动 Run 时，在看板上拖出 active 列、在详情面板改状态或改 assignee，先弹出确认框：

> Dispatcher is running TASK-xxx (implement). It will stop after the current stage ends; the agent process is not killed and the worktree is kept. Continue?

确认后照常写 ticket，卡片随即变为 stopping。改标签、优先级、正文不弹确认（不触发静默停止）。

### 对并行票的假设

- [[TASK-2nt8u]]（强制单 Dispatcher）：假设它的结果是项目级锁，第二个 serve 不能打开扫描开关；拒绝原因通过扫描开关的禁用状态和提示显示，复用 [[TASK-cq846]] "拒绝并说明原因"的入口。**附带要求**：同一 Project 的 Run 记录如果正被另一个仍在运行的 Dispatcher 持有，本 serve 的 `/api/runs` 不能把这些非终态记录误判为 `interrupted`，应显示为 `running elsewhere`（灰色，无时长），并且不提供恢复操作位。
- [[TASK-pd8bj]]：interrupted 的 Run 是否占名额、孤儿进程怎么处理由它决定；本票按"不占名额、不列入总览"展示。如果它改为占名额，总览要把 interrupted 的 Run 也列出来。

### 自问自答（均按推荐采纳）

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 1 | 界面的 Run 状态从哪里来，要不要塞进 `TaskResponse`？ | 只来自 Dispatcher（内存加 `.runs/`），用单独的 API；不进 `TaskResponse` |
| 2 | 用 assignee 是 `dispatcher` 判断"运行中"可以吗？ | 不可以，assignee 在 Run 结束后仍保留；以 Run 的显示状态为准 |
| 3 | 卡片要区分哪些状态？ | running（按 Stage）、merge-queued、stopping、interrupted、failed、stopped；succeeded 不显示 |
| 4 | interrupted 由谁判定？ | 服务端判定：记录非终态但内存里没有这个 Run |
| 5 | 标记显示什么时长？ | 当前 Stage 已用时长，按分钟取整，客户端每 30 秒刷新；总用时放在悬停提示里 |
| 6 | 标记放在哪、能不能单独点？ | 卡片底部 assignee 行右侧；不单独可点，点卡片打开详情面板 |
| 7 | 详情面板怎么展示 Run？ | 元数据和正文之间加 Run 段：Stage 进度、失败原因、Details 折叠区、Earlier runs 折叠区、操作位 |
| 8 | 要不要独立的 Dispatcher 总览页面？ | 不要；做成看板顶部名额徽章的弹出层，按 Project 分组 |
| 9 | 名额占用怎么展示？ | `Slots 12/20`，包含排队等 merge 的 Run，紧挨扫描开关；Settings 的全局卡片显示 `In use` |
| 10 | 着陆页、侧栏、Project 切换器要不要 Run 角标？ | 都不要 |
| 11 | 推哪些事件？ | `run_changed`（按 Project）和 `dispatcher_changed`（发到所有 Project 通道），薄事件加查询失效 |
| 12 | 事件粒度多细？ | 每次写 Run 记录之后发一次；不推心跳和 agent 输出 |
| 13 | 断线期间丢了事件怎么办？ | 重连后让全部查询失效 |
| 14 | Run 失败要不要主动提醒？ | 当前 Project 弹 toast 并附 "Open"；不做桌面通知，成功不提示 |
| 15 | 其他列表要不要显示 Run 标记？ | Needs You 行和 Dashboard Active Work 复用 `RunBadge`；Map、Roadmap 不显示 |
| 16 | 运行中人改状态或 assignee 要不要确认？ | 要，弹出确认框说明"当前 Stage 结束后停止、不杀进程"；改其他字段不确认 |
| 17 | 界面要不要"终止 Run"按钮？ | 不要 |
| 18 | "恢复执行"留在哪？ | 留两个位置：Run 段操作位、名额徽章旁；由 [[TASK-pd8bj]] 选 |
| 19 | 在另一台机器上跑过的 ticket 怎么显示？ | 没有卡片标记；详情面板显示一行 "No local Run record" |
| 20 | 文案用中文还是英文？ | 跟随看板，用英文 |

## Comments

### 2026-10-10 chuck

后续决策修订：
- [[TASK-pd8bj]]：确认被中断的 Run 不占名额、不列入总览；两个预留操作位都用上：Run 段放 `Resume`，名额徽章旁放当前 Project 的 `Resume N interrupted`（有中断的 Run 时才出现）。
- [[TASK-kbyh2]]：Run 段在每个 Stage 旁显示 token，Details 折叠区显示每次调用明细与 Run 合计；扫描开关被 Dispatcher 自动关闭时显示原因（经 `dispatcher_changed` 推送）。
- [[TASK-a8246]]：`merge-queued` 的位次按工作树队列计，同一工作树的多个 Project 共用一套编号；等另一个 serve 时显示 #1，悬停提示 "waiting for another server" 并附持有者。
