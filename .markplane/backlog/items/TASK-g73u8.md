---
id: TASK-g73u8
title: agent CLI 无头调用契约
status: done
priority: medium
type: research
effort: medium
epic: null
plan: null
map: MAP-8aavc
depends_on: []
blocks:
- TASK-6tbmz
- TASK-pd8bj
- TASK-by9nc
- TASK-kbyh2
related: []
assignee: chuck
tags: []
position: af
created: 2026-10-10
updated: 2026-10-10
---

# agent CLI 无头调用契约

## Question

Cursor `agent` CLI（本机 2026.10.01）作为无人值守子进程时的调用契约是什么？需要查清：

- `-p --output-format stream-json` 输出的事件结构：会话 / chat ID 在哪个字段、最终结果事件长什么样、如何从中读出最后一行文本（用于 `REVIEW: clean|issues` 判定）。
- `--resume <chatId>` 能否与 `-p` 联用、追加一条新 prompt 继续会话。
- `-p` 模式下如何让 agent 使用某个技能（如 `implement`）：技能是否加载、用 `/implement TASK-xxx` 还是自然语言。
- 无人值守需要的标志：`--force` / `--trust` / `--approve-mcps` / `--sandbox`，以及 `--workspace` 指向 worktree 时的行为。
- 退出码语义（成功、模型错误、被中断）。
- 会话 jsonl 文件的落盘位置：当 workspace 是 `<repo>/.worktrees/<name>` 时，`~/.cursor/projects/<slug>/agent-transcripts/` 的 slug 如何推导；jsonl 的行格式；会话标题从哪里来。

## Answer

调研稿：分支 `research/agent-cli-contract`，文件 `research/agent-cli-contract.md`，worktree `/Users/chuck/agent-space/ammo-bot/.worktrees/research-agent-cli`（仅本地提交 `b0f4143`）。依据：`agent` 2026.10.01 的 `--help`、cursor.com/docs/cli 官方文档、CLI 打包源码，以及 10 次一句话的本机实验。

- **stream-json**：每行都有 `session_id`；第一行是 `system/init`，可从这里读 ID；成功时末行是 `result/success`（`is_error:false`，带 `usage`）。实测会输出 `thinking` 事件（文档说会被屏蔽），解析时要忽略未知 `type`。
- **最后一行文本**：取最后一个 `assistant` 事件的 text，再取最后一个非空行匹配 `REVIEW: clean|issues`；不要用 `result` 字段，它把各段 assistant 文本直接拼接，中间没有分隔符。
- **续跑**：`-p --resume <id> "<新 prompt>"` 可用，`session_id` 不变，上下文保留。**坑**：ID 不存在或 `--workspace` 不同时不报错、退出 0，静默新开空会话。续跑前要校验 `<configDir>/chats/<md5(workspace)>/<id>/meta.json` 存在，并用和第一次完全相同的 `--workspace` 和环境变量（`HOME`/`XDG_CONFIG_HOME`）。`agent create-chat` 可以预先拿到 ID（可选）。
- **技能**：`-p` 会加载用户级（`~/.cursor/skills`、`~/.claude/skills` 等）和项目级技能。`implement` 设置了 `disable-model-invocation: true`，只能写 `/implement TASK-xxx`，CLI 会把技能全文内联进用户消息；自然语言不会触发。
- **标志**：`-p --output-format stream-json --trust --force --workspace <worktree 绝对路径> --model <m>`，建议加 `--approve-mcps --sandbox disabled`。不用 `-w/--worktree`（它建在 `~/.cursor/worktrees/`）。本机不加 `--trust` 也能跑，是因为个人配置 `approvalMode: unrestricted`，不能依赖这一点。
- **退出码**：成功 0；模型非法、工作区未受信任时 1（只写 stderr）；SIGINT 130，SIGTERM 143（没有 `result` 事件）。成功 = 退出码 0 且有 `result/success`。
- **jsonl 位置**：`${CURSOR_DATA_DIR:-~/.cursor}/projects/<slug>/agent-transcripts/<id>/<id>.jsonl`，`slug = 非 [A-Za-z0-9] 全换成 - → 合并连续 - → 去掉首尾 -`。例：`.../ammo-bot/.worktrees/TASK-abc12` → `Users-chuck-agent-space-ammo-bot-worktrees-TASK-abc12`。映射有损，要由路径正向计算。
- **jsonl 行格式**：`{"role":"user"|"assistant","message":{"content":[text|tool_use{name,input}]}}`，末行 `{"type":"turn_ended","status":"success"|"aborted"}`；不记录工具结果和 thinking。文件是增量重建的快照，要整文件重读。
- **标题**：jsonl 里没有；在 `<configDir>/chats/<md5(workspace)>/<id>/meta.json` 的 `title`（`configDir = $CURSOR_CONFIG_DIR || $XDG_CONFIG_HOME/cursor || ~/.cursor`）。`-p` 会话不生成标题（store 里是 `"New Agent"`），由 Dispatcher 自己定。

## Comments

### 2026-10-10 chuck

本批决策追加的待查事实：
- [[TASK-2vpdd]]：给出用 agent 工作目录（`<repo>/.worktrees/<TASK-id>`）推导 transcript slug 的可靠规则；如果不可靠，Run 记录会额外存解析好的 transcript 绝对路径。
- [[TASK-cq846]]：确认 `agent models` 的输出格式（每行 `slug - label`）是否稳定、有没有更适合机器读取的格式；设置页的模型下拉要靠它。
