---
id: TASK-2vpdd
title: Run 记录存放位置
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-8aavc
depends_on:
- TASK-k6d9y
blocks:
- TASK-pd8bj
- TASK-by9nc
related: []
assignee: chuck
tags: []
position: ah
created: 2026-10-10
updated: 2026-10-10
---

# Run 记录存放位置

## Question

Run 记录（当前 Stage、worktree 路径、各 Stage 的会话 ID、结果、时间）存在哪里？选项：`.markplane/` 下随 git 提交、`.markplane/` 下 gitignore 的派生目录、或用户配置目录 `~/.config/markplane/`。需兼顾：serve 重启后可恢复、会话记录弹窗能按 ticket 找到会话、多台机器 / 多人协作时是否该共享。

## Answer

## Answer

### 结论

Run 记录存在**项目内、不进 git 的专用目录** `.markplane/.runs/`，每个 ticket 一个 JSON 文件 `.markplane/.runs/<TASK-id>.json`。它是**本机**的 Run 状态，只由 Dispatcher 写，不跨机器共享。需要跨机器、给人看的事实（失败的 Stage、原因、会话 ID、合并 SHA、是否推送）照 [[TASK-k6d9y]] 写进 ticket 评论，跟着 ticket 文件提交。

### 为什么选这里

| 候选 | 结论 | 理由 |
| --- | --- | --- |
| `.markplane/` 下随 git 提交 | 否 | worktree 路径、会话 ID、jsonl 会话记录都只在本机有意义；每个 Stage 都要写，会产生噪声提交，还会和 merge Stage 冲突；[[TASK-k6d9y]] 已定中间态不提交、同一仓库默认只有一个 Dispatcher |
| `.context/`（gitignore 的派生目录） | 否 | `.context/` 是 `sync` 能重新生成的派生产物；Run 记录无法从任何文件推导，丢了就没了，不能和"可重建"的东西放一起 |
| `~/.config/markplane/` | 否 | 要用注册表 id 或路径做键；Project 移走、移出注册表再加回、id 变化时记录就对不上了；而"Project 注册表"按定义不碰 Project 文件，Run 却是 Project 内的事（worktree 也在项目 `.worktrees/`） |
| **`.markplane/.runs/`（自带 gitignore）** | **是** | 跟着 Project 目录走；照 `.locks/` 的做法，目录里自写一个内容为 `*` 的 `.gitignore`，老项目不用迁移；agent 用 `MARKPLANE_ROOT` 指向主工作区，路径解析一致 |

已核实的代码事实：serve 的文件监听只对 `.md` 和 `config.yaml` 推送事件，所以 `.json` 写入不会触发 `file_changed`；没有代码会遍历 `.markplane/` 根目录，新加一个点目录不影响列表、索引和 `sync`。

### 记录模型

- **一个 ticket 一个文件**，里面是 `runs` 数组，新的 Run 追加在末尾。重新拾取（从头开始）就追加一个新 Run，旧 Run 及其会话保留，弹窗仍能看到。
- **Run 字段**：序号（文件内从 1 递增）、状态（`running` / `merge-queued` / `succeeded` / `failed` / `stopped`）、当前 Stage、worktree 绝对路径、分支、基点 SHA、merge 目标分支、合并提交 SHA、是否已推送、开始和结束时间、失败原因（对应六条失败判定之一）。
- **Stage 调用（attempts）**：每启动一次 agent 进程就追加一条：Stage（implement / fix / merge）、实际使用的模型（快照，不随后续设置变化）、会话（chat）ID、agent 的工作目录（用来推导 transcript 位置）、进程 PID、开始和结束时间、退出码、末行判定（`REVIEW: …` / `FIX: …`）、结果（ok / failed / timeout / interrupted）。同一 Stage 可以有多条调用（续跑、comment and continue），弹窗按会话 ID 分组。
- **不复制会话内容**：只存指针（会话 ID 加工作目录），弹窗按 [[TASK-g73u8]] 查到的 slug 规则去 `~/.cursor/projects/<slug>/agent-transcripts/` 读。worktree 删除后 jsonl 仍在 `~/.cursor`，所以指针依然有效；找不到时显示"会话记录不可用"。
- **不存 ticket 的状态、标签、assignee**：这些以 ticket 为准（[[TASK-k6d9y]]）。两者不一致时，以 ticket 为准，Run 记录只描述 Run 本身。

### 写入规则

- **只有 Dispatcher（serve 进程）写**；agent 不读也不写。用临时文件加 rename 原子写入；进程内每个 ticket 一把互斥锁就够了，因为 serve 本身有 `serve.pid` 单实例锁。
- **先写再动手**：开 Stage 前先写"调用已开始"，拿到会话 ID 后立即补上，结束时写结果。运行期间以内存状态为准，文件是持久化镜像。
- **serve 关闭不专门写"中断"**：重启时，凡是状态非终态（`running` / `merge-queued`）的记录一律视为"被中断"。崩溃和正常关闭走同一条路。
- **静默停止**（人在运行中改了 ticket）写 `stopped`，并保留 worktree 路径。

### 读取方

- **重启恢复**：serve 启动时遍历注册表里每个 Project 的 `.runs/*.json`，找出非终态 Run 交给"恢复执行"（具体怎么恢复由 [[TASK-pd8bj]] 定）。
- **会话记录弹窗**：按 ticket id 直接打开 `<TASK-id>.json`。在另一台机器上没有记录时，显示"本机没有该 ticket 的 Run 记录"，只靠评论里的会话 ID 提供线索。
- **保留期限**：本版本不自动删除，文件很小。随 ticket 归档或删除一起清理的规则交给 [[TASK-6hqkh]]。

### 前提假设

- [[TASK-cq846]] 无论把设置放在 `config.yaml` 还是 `projects.json`，都不影响本答案；Run 记录只快照"实际用的模型和超时"。
- [[TASK-g73u8]] 会给出 transcript slug 的推导规则；如果推导不可靠，就在调用里额外存解析好的 transcript 绝对路径（字段预留）。

### 自问自答（均按推荐采纳）

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 1 | Run 记录和 ticket 评论怎么分工？ | 记录是给机器看的本机状态；评论是给人看、可共享的摘要；状态、标签、assignee 只在 ticket 里 |
| 2 | 要不要跨机器 / 多人共享？ | 不共享；需要共享的事实走评论，随 ticket 提交 |
| 3 | 项目内还是用户配置目录？ | 项目内 |
| 4 | 放 `.context/` 吗？ | 不放；用独立的 `.markplane/.runs/`，像 `.locks/` 一样自带 `.gitignore` |
| 5 | 按 Run 还是按 ticket 分文件？ | 按 ticket 一个文件，内含 Run 数组，重新拾取时追加 |
| 6 | 用什么格式？ | JSON（纯机器读写，文件监听会跳过它，也不会被当成 Markplane item） |
| 7 | 存会话内容还是指针？ | 存指针（会话 ID、工作目录，必要时加 transcript 路径） |
| 8 | 同一 Stage 多次调用怎么记？ | 每次启动 agent 进程追加一条调用，弹窗按会话 ID 分组 |
| 9 | 谁写、何时写？ | 只有 Dispatcher 写；每次状态转换先写再执行；原子写入 |
| 10 | serve 关闭时要写"中断"吗？ | 不写；启动时把非终态记录一律视为中断 |
| 11 | 要不要存 agent 的 PID？ | 存，供重启时识别孤儿进程（怎么处理由 [[TASK-pd8bj]] 定） |
| 12 | 记录和 ticket 不一致时以谁为准？ | 以 ticket 为准 |
| 13 | 记录保留多久？ | 不自动删除；清理规则交给 [[TASK-6hqkh]] |
| 14 | 在另一台机器打开弹窗会怎样？ | 显示"本机没有 Run 记录" |

## Comments

### 2026-10-10 chuck

后续决策修订（[[TASK-2nt8u]]）：保证 `.runs/` 只有一个写入者的是仓库级 Dispatcher 锁，不是 `serve.pid`；打开扫描开关时，在已有检查之外再加一项取锁检查。

### 2026-10-10 chuck

后续决策修订（[[TASK-kbyh2]]）：每次 Stage 调用的记录增加 `usage`（未知时为 null）、`duration_api_ms` 和 stderr 末尾片段。
