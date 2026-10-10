---
id: TASK-kbyh2
title: 无人值守的权限与成本边界
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-8aavc
depends_on:
- TASK-g73u8
blocks: []
related: []
assignee: chuck
tags: []
position: ap
created: 2026-10-10
updated: 2026-10-10
---

# 无人值守的权限与成本边界

## Question

无人值守的 agent 会话权限与成本边界是什么？

- Dispatcher 给 agent 传哪些标志（`--force` / `--trust` / `--approve-mcps` / `--sandbox` 等），允许 agent 做到什么程度：能否联网、能否执行任意命令、能否改 worktree 以外的文件。
- 除每 Stage 超时（默认 60 分钟）外，是否需要用量 / 成本上限（每 Run、每天、每 Project），超限时 Run 怎么处理。
- 依赖 [[TASK-g73u8]] 查清的各标志的实际行为。

## Answer

## Answer

### 结论

agent 会话以**完全权限**运行，没有沙箱：可以联网、执行任意命令、读写本机用户能读写的任何文件。边界靠三样东西：只拾取带 `ready-for-agent` 的 ticket、prompt 里的行为约定、用户自己在 agent 配置里写的 `permissions.deny`。本版本**不设 token / 成本上限**，花费由结构性限制兜底：并发上限 × 每 Stage 超时 × 每个 Run 最多 3 个 Stage，加上扫描开关每次启动都是关。花钱的硬上限交给 Cursor 账号自己的消费上限。用量按 Stage 调用记进 Run 记录，在详情面板展示。另加一个保护：同一 Project 连续 3 次 agent 启动失败时，Dispatcher 自动关闭该 Project 的扫描开关。

### 依据的事实（本机 `agent` 2026.10.01）

- `--force`：允许所有命令，除非被明确拒绝（`permissions.deny`）；不加时改动只提议、不落盘。在官方 Run Modes 文档里，"Run Everything" 模式没有沙箱、没有分类器。
- 沙箱开启时（macOS 用 Seatbelt）：工作区内可读写；工作区外不能写；`.git/config`、`.git/hooks` 等路径受保护；网络默认拒绝，只放行一份包管理器域名清单；`127.x` / 私网地址一律拦截。放宽要在 `sandbox.json` 里配置（用户级 `~/.cursor/` 或仓库内 `.cursor/`）。
- worktree 里提交会写 `<repo>/.git/worktrees/<id>` 和 `<repo>/.git/objects`，都在 worktree **之外**；agent 写评论要写主工作区的 `.markplane/`；merge Stage 要动主工作区；cargo / npm 还要写 `~/.cargo`、`~/.npm`。开沙箱时这些都会被拦。
- `--trust` 把信任标记写到 `~/.cursor/projects/<slug>/.workspace-trusted`（和 transcript 同一个目录），**不会写进 worktree**，不会弄脏分支。
- `--approve-mcps`：自动批准所有 MCP server，包括用户级 `~/.cursor/mcp.json` 和 workspace 的 `.cursor/mcp.json`。
- `--auto-review`：分类器拦下的调用要等人批准。无人值守时没人能批，所以不能用。
- `result` 事件带 `usage`：`inputTokens` / `outputTokens` / `cacheReadTokens` / `cacheWriteTokens`，还有 `duration_api_ms`。只在会话结束时出现一次。超时、被杀、崩溃时没有 `result` 事件，也就没有用量。stream-json 里没有金额。
- `agent models`：见文末"`agent models` 输出格式"。

### 无人值守调用契约

固定参数，不提供可配置的附加参数：

```bash
agent -p --output-format stream-json \
  --trust --force --approve-mcps --sandbox disabled \
  --workspace <该 Stage 工作目录的规范化绝对路径> \
  --model <Stage 模型快照> \
  [--resume <session_id>] "<prompt>"
```

| 项 | 决定 |
| --- | --- |
| 不传的参数 | `--auto-review`（会等人批准）、`--mode` / `--plan`（只读）、`-w/--worktree`（会建到 `~/.cursor/worktrees/`）、`--add-dir`、`--api-key`、`--stream-partial-output` |
| 沙箱 | 始终 `--sandbox disabled`，不跟随用户配置，不提供开关 |
| 认证 | 用 serve 所属用户的 `agent login` 会话；不提供 API key 设置（`config.yaml` 是共享文件，不放任何秘密） |
| 环境变量 | 继承 serve 进程的完整环境，不做过滤；另外加 `MARKPLANE_ROOT=<主工作区>`（[[TASK-k6d9y]]）和 `MARKPLANE_AUTHOR=agent-<stage>`（[[TASK-z97fe]]）；不改 `CURSOR_CONFIG_DIR` / `HOME`（改了会话存储和认证就会换位置，见 [[TASK-g73u8]]） |
| 进程 | cwd = 工作目录；stdin 接 `/dev/null`，没有任何东西能卡在等输入上；每个 agent 单独一个进程组。超时先给整个进程组发 SIGTERM，10 秒后还在就 SIGKILL，这样 agent 起的子进程也会一起结束 |
| 用户的权限配置 | Dispatcher 不注入、也不覆盖 agent 的 CLI 配置；用户的 `permissions.deny`（`cli-config.json` 或仓库的 `.cursor/cli.json`）在 `--force` 下依然生效（文档如此，未实测）。这是唯一的硬限制手段，写进文档 |

### agent 能做到什么程度

| 能力 | 技术上 | 约定（写在 prompt 里，[[TASK-6tbmz]]） |
| --- | --- | --- |
| 联网 | 完全放开 | 只为构建、安装依赖、查文档联网 |
| 执行命令 | 任意命令（除了 `permissions.deny` 里的） | 不起常驻进程；不交互 |
| 写 worktree 内 | 可以 | implement / fix 的主要工作区 |
| 写 worktree 外 | 本机用户能写的都能写 | implement / fix 只允许两种：git 在 common dir 里的正常写入（提交到自己的 `agent/<TASK-id>` 分支），以及用 `markplane comment` 往主工作区追加评论（靠 `MARKPLANE_ROOT`）。merge Stage 可以在主工作区做合并，这是它本来的工作 |
| git | 什么都能做 | 只操作自己的分支；**不 push**（推送由 Dispatcher 在提交 ticket 文件后按开关执行）；不碰其他 worktree 和分支；不改 git config |
| ticket | 能改主工作区任何文件 | 只追加评论；不执行 `markplane done` / `status` / `update`（[[TASK-k6d9y]]） |
| MCP | 全部自动批准 | 不需要；想禁止的 server 用 `permissions.deny` 里的 `Mcp(server:*)` |

越界写入不做检测，也不拦截。唯一的兜底是 [[TASK-k6d9y]] 第 5 条失败判定：主工作区有未提交改动挡住合并时，merge 失败（agent 写脏主工作区也会被这一条抓到）。

### 用量记录与展示

- **记录**：在 Run 记录的每次 Stage 调用（[[TASK-2vpdd]]）里加 `usage`，四个字段原样照抄，外加 `duration_api_ms`；没有 `result` 事件时记 `null`，表示"未知"，不是 0。失败时同时存 stderr 末尾一段（[[TASK-g73u8]] 建议），作为失败原因的补充。
- **展示**：在详情面板的 Run 段（[[TASK-exz6h]]），Stage 进度每一步在模型和用时旁边加上 `in / out` token（缩写，如 `1.2M in · 26k out`）；Details 折叠区列出每次调用的四个字段和本 Run 的合计（有调用的用量未知时，合计前加 `≥`）。卡片、名额徽章总览、ticket 评论里都不显示用量。不换算成金额：没有价格数据，计费方式也因账号而异。

### 上限与超限

| 问题 | 决定 |
| --- | --- |
| 每 Run / 每天 / 每 Project 的 token 或金额上限 | **都不设**。用量只在会话结束时才知道，没法在 Stage 中途叫停；一个 Run 最多 3 个 Stage、fix 只有一轮，在 Stage 之间做每 Run 上限意义不大；花费已经被并发上限（默认 20）、每 Stage 超时（默认 60 分钟）、Stage 数量、扫描开关启动即关、只拾取 `ready-for-agent` 这几条限制住 |
| 花钱的硬上限 | 用 Cursor 账号的消费上限（它管这个账号的所有用量）；写进文档 |
| 账号额度用尽 / 未登录 / 模型不可用 | 按普通 Stage 失败处理（非零退出，或没有 `result/success`），ticket 转人工（[[TASK-k6d9y]]）；不另设状态 |
| 连续启动失败保护 | 同一 Project 连续 **3 次** Stage 调用在拿到 `system/init` 之前就结束（启动失败：未登录、模型不可用、未受信任，可能也包括额度用尽，最后一项未核实），Dispatcher 就关闭该 Project 的扫描开关：在内存里记下原因，写 `serve.log`，发 `dispatcher_changed`，界面在开关旁显示原因。正在跑的 Run 继续跑完。只要有一次调用拿到了 `init`，计数就清零。阈值写死，不提供配置。触发前已经失败的那几张 ticket 照常转人工（并发高时，开关关上之前可能同时失败多张，可以接受） |
| 配置位置 | 本票**不新增任何设置**，[[TASK-cq846]] 的分层不变。以后如果要加上限：按账号算的放 `serve.json`（本机层），按 Project 算的放 `config.yaml` 的 `dispatcher` 段 |

### 文档里的安全说明（写进 Dispatcher 相关文档；Settings 的 Dispatcher 标签页加一行静态提示）

1. Dispatcher 以 serve 所属的系统用户身份、在**没有沙箱**的情况下运行 agent：可以联网、执行任意命令、读写该用户能访问的任何文件，自动批准所有 MCP server，能看到 serve 进程的全部环境变量（包括其中的密钥）。
2. ticket 的正文和评论就是 prompt。谁能往仓库里提交带 `ready-for-agent` 的 ticket（包括 `git pull` 拉下来的），谁就能让你的机器执行命令。只在可信的仓库上打开扫描开关。
3. 建议的加固手段：用专用系统用户 / 虚拟机 / 容器运行 serve；在 agent 的 `permissions.deny` 里禁掉危险命令和不需要的 MCP；不要把密钥放进 serve 的环境变量；在 Cursor 账号上设消费上限。
4. 花费由并发上限 × 每 Stage 超时 × 每个 Run 最多 3 个 Stage 限制；Dispatcher 本身不设 token 上限。
5. agent 被约定不 push、不改 ticket 状态，但没有技术手段强制执行。

### `agent models` 输出格式（回应 [[TASK-cq846]]）

- `agent models` 和 `agent --list-models` 输出完全相同，都是纯文本：第 1 行 `Available models`，空一行，然后每行一个 `<slug> - <label>`（本机 256 行），再空一行，最后一行是 `Tip: ...`。
- **没有机器可读格式**：加 `--output-format json` 会被忽略，照样输出文本；`--help` 里也没有相关选项；官方没有承诺输出格式稳定。
- label 可能带标记后缀：` (default)`（本机是 `auto`）、` (current)`（当前选中的模型，本机是 `cursor-grok-4.5-high`）。
- 管道输出时没有颜色；设了 `FORCE_COLOR` 就会带 ANSI 转义码。
- slug 都匹配 `[a-z0-9][a-z0-9.-]*`；带参数的写法（如 `claude-opus-4-8[...]`）不会列出来。
- 要联网查账号，耗时约 2 秒，需要已登录。
- 解析建议：设 `NO_COLOR=1` 并去掉 ANSI 码，逐行匹配 `^(\S+) - (.+)$`，去掉 label 末尾的 `(default)` / `(current)`，其余行忽略；在 serve 内缓存，比如 10 分钟；解析失败或命令失败时下拉框为空、只能自由输入（[[TASK-cq846]] 已允许自由输入）。另外，`--model` 非法时 stderr 也会列出可用模型，但同样是文本。

### 自问自答（均按推荐采纳）

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 1 | 用固定参数 `-p stream-json --trust --force --approve-mcps --sandbox disabled --workspace --model`，不提供附加参数设置？ | 是 |
| 2 | 始终关闭沙箱，不提供开关？ | 是。worktree 提交、写评论、merge、构建缓存都要写工作区以外的地方，开沙箱就要在仓库里放 `sandbox.json` 并实测，本版本不做 |
| 3 | agent 能联网、能执行任意命令？ | 能；靠 `permissions.deny` 和 prompt 约束 |
| 4 | worktree 外的写入只靠约定（git common dir、`markplane comment`、merge 写主工作区），不检测？ | 是；只靠 k6d9y 第 5 条兜底 |
| 5 | agent 不 push，由 Dispatcher 推送？ | 是 |
| 6 | 传 `--approve-mcps`？ | 是；无人值守时没人能批准，和 `--force` 相比增加的风险很小 |
| 7 | 不注入、不覆盖 agent 配置，尊重用户的 `permissions.deny`？ | 是 |
| 8 | 继承 serve 完整环境，只额外加 `MARKPLANE_ROOT` / `MARKPLANE_AUTHOR`？ | 是 |
| 9 | stdin 接 `/dev/null`，单独进程组，超时 SIGTERM，10 秒后 SIGKILL？ | 是 |
| 10 | 只用 `agent login`，不在配置里放 API key？ | 是 |
| 11 | 每次 Stage 调用记 `usage` 和 `duration_api_ms`，没有就记 `null`？ | 是 |
| 12 | 在详情面板 Run 段显示 token，卡片和评论里不显示，不换算金额？ | 是 |
| 13 | 不设每 Run / 每天 / 每 Project 的上限？ | 是；靠结构性限制和 Cursor 账号消费上限 |
| 14 | 额度用尽等情况按普通 Stage 失败处理？ | 是 |
| 15 | 同一 Project 连续 3 次启动失败就自动关闭扫描开关并显示原因？ | 是；阈值写死 |
| 16 | 本票不新增设置？ | 是 |
| 17 | 写安全说明文档，Settings 里加一行提示，不加确认弹窗？ | 是 |
| 18 | `agent models` 按文本解析并缓存，失败时退回自由输入？ | 是 |
