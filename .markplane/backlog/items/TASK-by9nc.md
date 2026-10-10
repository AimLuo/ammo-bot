---
id: TASK-by9nc
title: 会话记录弹窗
status: done
priority: medium
type: prototype
effort: medium
epic: null
plan: null
map: MAP-8aavc
depends_on:
- TASK-g73u8
- TASK-2vpdd
blocks: []
related: []
assignee: chuck
tags: []
position: am
created: 2026-10-10
updated: 2026-10-10
---

# 会话记录弹窗

## Question

"查看会话记录"弹窗应长什么样、如何工作？屏幕中央弹窗展示一个 ticket 的各 Stage 会话（标题、会话 ID），点击展开会话历史（读 agent 的 jsonl），带主动刷新按钮。需要定：多个 Stage / 多次续跑的会话如何排列、历史里展示哪些消息类型（用户 / 助手 / 工具调用）、长会话如何截断。做一个粗糙原型让用户评判。

## Answer

原型：分支 `prototype/session-modal`（本地提交 `8a0a763`），代码在 `crates/markplane-web/ui/src/components/domain/session-log-prototype/`（含 3 个变体、固定数据和 `build_transcripts.py`）、共享切换条 `src/components/prototype-switcher.tsx`；worktree `.worktrees/prototype-session-modal`，`npm run prototype:session-log`。用户评审：选 **Variant A**，细节倾向全部接受。

### 结构（Variant A：主从布局）

- 大尺寸居中弹窗：左侧列表，右侧所选会话的完整记录。
- 左侧排序：最新的 Run 在最前；Run 内 Stage 按流程顺序（implement → fix → merge）；同一 Stage 下按会话 ID 分组，组下按时间列出各次调用（首次 / resume / continue），标出触发方式与 `resume_mismatch`。
- 右侧默认显示**整个会话**，用分隔线标出每次调用的边界；点某次调用时滚到对应位置。
- 消息类型：Dispatcher 发的 prompt、agent 文本、工具调用；可分别开关。工具调用借用 Variant B 的做法：**每轮折成一行**，可展开看输入（jsonl 本身不含工具结果和 thinking）。
- 截断：显示最后 60 条消息，顶部"show earlier / show all"。
- 刷新：头部一个手动 Refresh 按钮；弹窗不响应 `run_changed`。
- 用量：每次调用显示输入 / 输出 token 文本，会话与 Run 显示合计；缓存明细放悬停提示。

### 入口与空状态

- 入口：详情面板 Run 段头部操作位的 **View sessions** 按钮（界面文案用英文，与 [[TASK-exz6h]] 一致）。
- 本机没有 Run 记录：Run 段仍显示一行灰色 "No local Run record" 和按钮，弹窗内显示对应空状态。
- 会话记录不可用（文件不存在或无法读取）：该会话条目保留，右侧显示不可用提示，不影响其他会话。
- 已归档 ticket、worktree 已删除的 Run 同样可用：只依赖 Run 记录里的会话指针。

## Comments

### 2026-10-10 chuck

本批决策带来的约束：
- [[TASK-2vpdd]]：弹窗按 ticket id 读 `.markplane/.runs/<TASK-id>.json`，按 Run、再按会话 ID 分组展示 Stage 调用；用会话 ID 加 agent 工作目录去 `~/.cursor/projects/<slug>/agent-transcripts/` 找会话记录；要设计"本机没有 Run 记录"和"会话记录不可用"两种空状态。
- [[TASK-z97fe]]：任务详情面板里评论区旁边留了位置，"查看会话记录"入口放哪由本票决定。

### 2026-10-10 chuck

第二批决策带来的约束：
- [[TASK-exz6h]]：数据可直接用 `GET /api/runs/{task_id}`；建议入口放在详情面板 Run 段头部的操作位；弹窗不响应 `run_changed`，保持只手动刷新。

### 2026-10-10 chuck

第三批决策带来的约束：
- [[TASK-pd8bj]]：同一会话 ID 下可能有多条调用（首次、resume、continue），要显示每条的触发方式，并标出 `resume_mismatch`。
- [[TASK-kbyh2]]：可选择显示每个会话的 token 用量（取自 Run 记录）。

### 2026-10-10 chuck

第四批决策带来的约束：
- [[TASK-6hqkh]]：弹窗要能用于已归档的 ticket 和 worktree 已删除的 Run，只依赖 Run 记录里的会话指针，不依赖 worktree 目录存在。
