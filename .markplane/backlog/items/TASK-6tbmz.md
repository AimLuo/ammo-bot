---
id: TASK-6tbmz
title: 各 Stage 的 prompt
status: done
priority: medium
type: prototype
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
position: aj
created: 2026-10-10
updated: 2026-10-10
---

# 各 Stage 的 prompt

## Question

implement / fix / merge 三个 Stage 各自发给 agent 的 prompt 具体怎么写？至少覆盖：implement 如何点名 `implement` 技能与 ticket、如何要求把 review 发现写成 ticket 评论并以 `REVIEW: clean|issues` 结尾；fix 如何定位要修的评论；merge 如何合并到主工作区、处理冲突、按开关推送，以及各 Stage 如何报告失败。还需满足"Run 生命周期与任务状态映射"[[TASK-k6d9y]] 定下的约束：fix 以 `FIX: done|blocked` 结尾；禁止 agent 执行 `markplane done` / `status` / `update`（只允许追加评论）；`implement` 技能里"引用不明确就提问"在无人值守下如何处理。做成可直接试跑的草稿让用户评判。

## Answer

原型：分支 `prototype/stage-prompts`（本地提交 `489d15f`），目录 `prototype/stage-prompts/`；worktree `.worktrees/prototype-stage-prompts`。模板在 `templates/`，公共片段在 `templates/partials/`，可用 `run-stage.sh <stage> <TASK-id> --dry-run` 试渲染。用户评审：全部接受。

### 结构

- 三个 Stage 模板 `implement.md` / `fix.md` / `merge.md`，加两个续跑模板 `resume-interrupted.md`、`continue-with-comment.md`（通过 `{{stage_contract}}` 槽复用各 Stage 的末行约定）。
- 公共片段：无人值守规则（从不提问、不起交互或常驻进程）、worktree 边界、ticket 规则（只用 `markplane comment` 追加，正文不用 H1–H3）、三个末行约定（输出末行前先提交或清掉所有改动）、两个新开会话兜底片段（说明 worktree 里可能已有做了一半的改动）。

### 定下的取舍

| 项 | 决定 |
| --- | --- |
| prompt 语言 | 英文（技能与判定词是英文）；agent 用 ticket 的语言写评论 |
| implement 点名技能 | `/implement {{task_id}}` 显式调用 |
| ticket 有歧义 | 不提问；按最合理假设继续，review 前写一条 `**Assumptions**` 评论 |
| 做不了 | 写 `**Blocked**` 评论，末行 `BLOCKED: <原因>`（不是合法 `REVIEW:` 行 → 按失败规则 2 转人工，不浪费 fix 轮） |
| review 发现 | implement 不自修；分 must-fix / optional，总写一条 `**Review**` 评论（clean 也写）；有 must-fix 即 `REVIEW: issues` |
| fix 定位评论 | Dispatcher 内联最新一条 `agent-implement` 写的 `**Review**` 评论；未内联时 agent 自行 `markplane show`（评论日期只到天，不按 Run 开始时间过滤） |
| fix 拒绝某条发现 | 仅当确信是误报，写进 `**Fix report**`，仍算 `FIX: done`；"难修"则 `FIX: blocked` |
| merge 末行 | `MERGE: done|failed`；非 `done` 即失败，Dispatcher 再核对目标分支确实包含 agent 分支 |
| merge 方式与测试 | `--no-ff --no-commit` 合并，提交前验证；干净合并只做 build/typecheck，手工解决过冲突才跑全量测试并写 `**Merge notes**` 评论 |
| merge 安全 | 禁止 stash / reset / clean / `git add -A`；`.markplane/` 下冲突一律保留主工作区版本；放弃时 `git merge --abort`；prompt 不提推送（由 Dispatcher 在合并成功那一刻决定） |

### 实现提示（进 Spec）

CLI 尚无 `MARKPLANE_ROOT` / `MARKPLANE_AUTHOR`；原型用放在 agent PATH 最前的 `markplane` 包装脚本代替：切到主工作区、按 Stage 署名，并拒绝除 `comment` 外所有改 ticket 的子命令。正式实现以环境变量为准，包装脚本的"拒绝改 ticket"思路可作为额外防线。

## Comments

### 2026-10-10 chuck

本批决策带来的约束：
- [[TASK-cq846]]：是否推送以 merge 成功那一刻主工作区 `config.yaml` 的 `dispatcher.push_after_merge` 为准，prompt 不能在 Run 开始时把这个值写死。
- [[TASK-z97fe]]：agent 用 `markplane comment` 写评论、不自己署名（Dispatcher 用 `MARKPLANE_AUTHOR=agent-<stage>` 署名）；评论正文不要以 H1–H3 标题开头（会被降成 H4）。

### 2026-10-10 chuck

第三批决策带来的约束：
- [[TASK-pd8bj]]：每个 Stage 要多两类 prompt：被中断后继续、带人工评论继续（评论原文内联，重申末行约定）。新开会话的兜底 prompt 要说明 worktree 里可能已有做了一半的改动；merge prompt 要能处理主工作区里本分支没合完的状态。
- [[TASK-kbyh2]]：prompt 写明边界：implement / fix 只写自己的 worktree，git 只动自己的 `agent/<TASK-id>` 分支，ticket 只通过 `markplane comment` 追加评论，不 `git push`（推送由 Dispatcher 在提交 ticket 文件后做），不交互等待、不起常驻进程；只有 merge 碰主工作区。
- [[TASK-a8246]]：merge 会话只在持有工作树合并锁时运行；放弃合并时要求 `git merge --abort` 复原工作树；推送与 ticket 提交在同一临界区，prompt 不能另开路径提前推送。

### 2026-10-10 chuck

第四批决策带来的约束：
- [[TASK-6hqkh]]：implement / fix / merge 的 prompt 都要求 agent 在输出末行判定前，把所有改动提交或清掉；否则遗留的未跟踪文件会挡住成功 Run 之后的自动删除 worktree。
