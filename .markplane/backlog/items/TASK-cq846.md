---
id: TASK-cq846
title: Dispatcher 设置模型
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
position: ai
created: 2026-10-10
updated: 2026-10-10
---

# Dispatcher 设置模型

## Question

Dispatcher 的设置放在哪、怎么改？包括：按 Project 的扫描开关（默认关）、合并后推送开关（默认开）、implement / fix / merge 各 Stage 的模型（默认 `cursor-grok-4.5-high`），以及 serve 全局的并发上限 20。候选位置：`.markplane/config.yaml`（随仓库共享）、用户级 `projects.json` 条目、或二者分工。界面上在哪里改（现有 `/api/config` 设置页、看板顶部开关等）。

## Answer

## Answer

### 分层原则

按"该不该随仓库共享"分三层，每项设置只在一处：

| 层 | 位置 | 放什么 | 理由 |
| --- | --- | --- | --- |
| 仓库共享 | 主工作区 `.markplane/config.yaml` 新增 `dispatcher` 段 | 合并后推送开关、各 Stage 模型与超时 | 属于项目策略，随 git 共享、可审阅；协作者看到同一套 |
| 本机 serve 全局 | 用户配置目录新文件 `serve.json`（与 `serve.pid` / `serve.log` 并列） | 全局并发上限 | 跨 Project 共享，不属于任何一个仓库；不塞进 `projects.json`（那是 Project registry，只列 Project） |
| 运行时状态 | serve 进程内存，不落盘 | 按 Project 的扫描开关 | 同一仓库只假设一个 Dispatcher：写进 `config.yaml` 会让每个克隆者的 serve 都开扫，且拾取状态不跨机器（见 [[TASK-k6d9y]]）；切换开关也不应弄脏工作树；不落盘同时满足"重启后不自动扫描" |

Dispatcher 读的是**主工作区**的 `config.yaml`，worktree 分支里对它的改动在合并前不生效。

### 设置项

| 设置 | 层 | 默认 | 取值 | 何时读取 |
| --- | --- | --- | --- | --- |
| 扫描开关 | 运行时 | 每次 serve 启动都是关 | 开 / 关 | 每次扫描（15 秒） |
| 合并后推送 | 仓库 | 开（本仓在自己的 `config.yaml` 里写关） | 布尔 | merge 成功、准备推送时 |
| implement / fix / merge 模型 | 仓库 | `cursor-grok-4.5-high` | 模型 slug | 该 Stage 开跑时 |
| implement / fix / merge 超时 | 仓库 | 60 分钟 | 1–1440 分钟整数 | 该 Stage 开跑时 |
| 全局并发上限 | 本机 serve | 20 | 1–100 整数 | 每次拾取判断时 |

`config.yaml` 形状（全部可省略，缺省即默认；未显式设置的项不写回文件，这样以后改默认值能直接生效）：

```yaml
dispatcher:
  push_after_merge: false
  stages:
    implement: { model: cursor-grok-4.5-high, timeout_minutes: 60 }
    fix:       { model: cursor-grok-4.5-high, timeout_minutes: 60 }
    merge:     { model: cursor-grok-4.5-high, timeout_minutes: 60 }
```

`serve.json`：`{ "dispatcher": { "concurrency": 20 } }`，缺文件即默认。

### 生效与语义

- 全部热生效，不需要重启 serve；手改 `config.yaml`（含 `git pull`）同样生效。
- 模型和超时在 Stage 开跑时取快照，运行中的 Stage 不受改动影响。
- 并发调低时不杀正在跑的 Run，只是占用降到上限以下之前不再拾取；调高在下一次扫描生效。
- 关扫描只停止拾取新 ticket；正在跑的 Run（含排队等 merge 的）照常跑完。
- 打开扫描前检查：Project 是 git 仓库、`agent` 在 PATH 上；不满足就拒绝并说明原因。

### 校验

- 模型：去掉首尾空白后非空、不含空白、≤200 字符。不在本机 `agent models` 列表里时只警告、不拒绝（`config.yaml` 是共享的，别人的账号可用模型可能不同）；真跑起来不可用，会按 Stage 失败处理（非零退出）。
- 超时 1–1440，并发 1–100；越界由 API 返回 400 `validation_error`，与现有 `/api/config` 一致。
- 手改出非法值：该 Project 沿用上一次有效的 Dispatcher 配置，并在 `serve.log` 记警告；`serve.json` 读不出就用默认 20 并记警告。

### 界面

- **看板顶部**（与 "Decision tickets" 开关同一行）：Dispatcher 扫描开关，按当前 Project。这是唯一入口，因为它最常切换。
- **Settings 新增 "Dispatcher" 标签页**，两张卡片：
  - "本 Project（随仓库共享，写入 config.yaml）"：推送开关；三个 Stage 各一个模型下拉（选项来自本机 `agent models`，可自由输入）和一个超时输入框。
  - "本机 serve 全局（所有 Project 共享）"：并发上限，每个 Project 的设置页显示同一个值。
- API：`/api/config` 的 GET / PATCH 加 `dispatcher` 段；扫描开关用按 Project 的新端点；并发上限和模型列表用不带 Project 的全局端点（与 `/api/projects` 同级）。
- 不新增 CLI 配置命令，手改 YAML 即可。

### 依赖与假设

- 本票只保证"serve 启动时所有扫描开关都关"。"恢复执行"要不要顺带打开扫描由 [[TASK-pd8bj]] 决定；如果需要记住重启前的开关状态，存放位置是 `projects.json` 里对应的 Project 条目（本机层），不进 `config.yaml`。
- 每个 Stage 实际用的模型和超时应写进 Run 记录，存放位置见 [[TASK-2vpdd]]。

### 自问自答（均按推荐采纳）

1. 按共享范围把设置分成仓库 / 本机 serve / 运行时三层？是。
2. 扫描开关只放在内存、不落盘、启动即关？是。
3. 合并后推送开关放 `config.yaml`，缺省开，本仓写关？是。
4. 各 Stage 模型放 `config.yaml`，不提供本机覆盖？是（暂不需要）。
5. 超时按 Stage 配，和模型放在一起，默认 60 分钟？是。
6. 全局并发放用户配置目录新建的 `serve.json`，而不是 `projects.json` 或 CLI 参数？是（CLI 参数在 daemon 重启时容易丢，界面也改不了）。
7. 全部热生效，Stage 开跑时取快照，调低并发不杀进程？是。
8. 关扫描只停拾取，运行中的 Run 跑完？是。
9. 缺省项不写回 `config.yaml`？是。
10. 扫描开关放看板顶部，其余放 Settings 的 Dispatcher 标签页，全局项单独一张卡片并标明？是（暂不做全局设置页）。
11. 模型用本机 `agent models` 做下拉、允许自由输入，不在列表里只警告？是。
12. 数值范围：超时 1–1440、并发 1–100？是。
13. 手改非法值时沿用上次有效配置并记日志？是。
14. 打开扫描前检查 git 仓库和 `agent` 可用？是。
15. 不加 CLI 配置命令？是。
16. 读主工作区的 `config.yaml`，不读 worktree 里的？是。

## Comments

### 2026-10-10 chuck

后续决策修订（[[TASK-2nt8u]]）：保证 `.runs/` 只有一个写入者的是仓库级 Dispatcher 锁，不是 `serve.pid`；打开扫描开关时，在已有检查之外再加一项取锁检查。

### 2026-10-10 chuck

后续决策修订（[[TASK-kbyh2]]）：
- 不新增设置；Settings 的 Dispatcher 标签页加一行安全说明。
- `agent models` 没有机器可读格式（`--output-format json` 被忽略）：输出为 `Available models`、空行、每行 `<slug> - <label>`、空行、`Tip:` 行；label 可能带 ` (default)` / ` (current)` 后缀；需联网并已登录，约 2 秒。解析：设 `NO_COLOR=1` 并去掉 ANSI，逐行匹配 `^(\S+) - (.+)$`，去掉后缀标记，缓存约 10 分钟，失败时退回自由输入。
