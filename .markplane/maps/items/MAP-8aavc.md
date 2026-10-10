---
id: MAP-8aavc
title: Agent Dispatcher 与任务卡片增强
status: done
specs:
- EPIC-24iwg
- EPIC-ce5ps
related: []
tags: []
created: 2026-10-10
updated: 2026-10-10
---

# Agent Dispatcher 与任务卡片增强

## Destination

两份可交给 `/to-spec` → `/to-tickets` 的 Spec（EPIC）：

1. **Agent Dispatcher**：定时扫描、worktree 并行 Run、implement / fix / merge 三个 Stage、失败转人工与 comment and continue、重启后人工恢复。
2. **任务卡片评论与会话查看**：评论文本域、查看会话记录弹窗、Needs You 标签页。

## Notes

- 领域：本仓 `markplane serve`（Tokio + Axum，多 Project 注册表）与 Next.js UI；agent 指 Cursor `agent` CLI。
- 每个会话先读 `GLOSSARY.md`（Dispatcher / Run / Stage / Needs You）；grilling 票同时用 grilling + domain-modeling 技能。
- 已定的前提（charting 时与用户确认）：
  - Dispatcher 跑在 `markplane serve` 进程内；"程序重启" = serve 重启。
  - 扫描对象：`planned` 状态、带 `ready-for-agent`、未被 blocked（复用 `find_blocked_items` 语义）的 Implementation ticket；每 15 秒一次。
  - 扫描开关按 Project，默认关；并发上限 20 为 serve 全局，跨 Project 共享。
  - 每个 Run 在项目 `.worktrees/` 下建独立 git worktree（`.worktrees/` 已在 `.gitignore`）。
  - "Codex review" = `implement` 技能末尾调用的 `code-review`，不是单独的阶段，也不接 Codex CLI。
  - implement 会话把 review 发现写成 ticket 评论，最后一行输出 `REVIEW: clean|issues`；`issues` 时在同一 worktree 起**新**的 fix 会话读评论修复。
  - 合并：每个工作树（Project 所在的 git 工作树）同一时刻只跑一个 merge Stage；同一工作树里的多个 Project 共用一条 merge 队列（跨 serve 由合并锁保证），排队也占全局名额（见 [[TASK-a8246]]）；冲突由 agent 解决，解决不了即失败并加 `ready-for-human`。
  - 合并后推送：按 Project 开关，默认开（本仓自身关闭）。
  - 每个 Stage 可单独配模型，默认 `cursor-grok-4.5-high`。
  - 重启后不自动扫描、不自动恢复；由人工点"恢复执行"。

## Decisions so far

- Run 生命周期与任务状态映射 [[TASK-k6d9y]]: 主工作区为唯一事实来源，只有 Dispatcher 改状态；拾取→in-progress+assignee dispatcher，成功→done 并只提交该 ticket，失败→留 in-progress、换 ready-for-human、保留 worktree；六条失败判定、fix 单轮
- Run 记录存放位置 [[TASK-2vpdd]]: Run 记录存项目内 gitignore 的 .markplane/.runs/<TASK-id>.json（一 ticket 一文件），只存本机、只由 Dispatcher 原子写；共享的事实走 ticket 评论
- Dispatcher 设置模型 [[TASK-cq846]]: 推送开关与各 Stage 模型/超时放 config.yaml；全局并发放本机 serve.json；扫描开关只在内存、启动即关；全部热生效，入口在看板顶部与 Settings 的 Dispatcher 标签页
- 评论功能与 Needs You 标签页 [[TASK-z97fe]]: 评论走专用 POST 追加、作者由服务端定，描述编辑不再覆盖评论区；Needs You 为 /backlog 第三个只读列表视图，含 Decision 票，排除完成/取消态
- agent CLI 无头调用契约 [[TASK-g73u8]]: agent -p stream-json 首行 init 含 session_id；--resume 可无头续跑但未知 ID/换 workspace 会静默新开空会话；/implement 显式调用；jsonl 在 projects/<slug>/agent-transcripts，-p 会话无标题
- Run 在界面上的呈现 [[TASK-exz6h]]: Run 状态由 Dispatcher 经独立 API 与 run_changed / dispatcher_changed 推给界面；卡片显示 Stage 与用时，看板顶部名额徽章弹出跨 Project 总览，详情面板加 Run 段
- 强制单 Dispatcher [[TASK-2nt8u]]: 每个 Project 一把放在 git common dir 的 OS 建议锁，开扫描或有活动 Run 时持有；冲突时只读并返回 409 及持有者；独立 clone 不管
- 重启恢复与 comment and continue [[TASK-pd8bj]]: 恢复与 continue 都续同一个 Run：能续跑就 --resume 原会话，否则同 Stage 新开；中断的 Run 不占名额，孤儿进程确认后再杀
- 无人值守的权限与成本边界 [[TASK-kbyh2]]: agent 固定传 --trust --force --approve-mcps --sandbox disabled，完全权限；不设 token/成本上限；Run 记录记每次调用的 usage；同 Project 连续 3 次启动失败自动关扫描开关
- 同一工作树内多个 Project 的 merge 串行 [[TASK-a8246]]: merge 按工作树串行：同工作树多 Project 共用一条 FIFO 队列，跨 serve 用 git dir 下的合并锁兜底，覆盖 ticket 提交与推送；名额规则不变
- 遗留 worktree 与分支的清理 [[TASK-6hqkh]]: 持锁对账时清理：ticket 完成或取消后自动删 worktree 与已合并分支，有未提交改动、未合并提交或孤儿进程时跳过；Run 段 Clean up 确认后强删；Run 记录永久保留
- 各 Stage 的 prompt [[TASK-6tbmz]]: 英文 prompt 模板 + 公共片段（原型 prototype/stage-prompts）；歧义写 Assumptions 不提问，做不了 BLOCKED 转人工；fix 读最新 Review 评论；merge 以 MERGE: done|failed 收尾、--no-ff --no-commit
- 会话记录弹窗 [[TASK-by9nc]]: 选 Variant A 主从弹窗（原型 prototype/session-modal）：左列 Run→Stage→会话→调用，右侧整段会话带调用分隔、工具调用每轮一行、末 60 条截断；入口为 Run 段 View sessions

## Not yet specified

## Out of scope

- 接入 OpenAI Codex CLI。
- 重启后自动恢复 Run。
- 富文本评论。
- 会话记录实时刷新（只做主动刷新按钮）。
- 跨机器 / 多人共享 Run 记录与会话内容；把会话 jsonl 复制进项目（只存指针）。见 [[TASK-2vpdd]]。
- 按机器覆盖各 Stage 模型；专门的 Dispatcher CLI 配置命令。见 [[TASK-cq846]]。
- 在 Epic / Plan / Note / Map 上从界面评论；界面编辑或删除评论；跨 Project 汇总的 Needs You 与侧栏角标。见 [[TASK-z97fe]]。
- 从界面终止正在运行的 agent；推送 agent 实时输出或心跳；桌面通知；单独的 Dispatcher 总览页及着陆页 / 侧栏 / Project 切换器上的 Run 角标。见 [[TASK-exz6h]]。
- 各自有 `.git` 的独立 clone 上的重复拾取（只在文档写明）；网络盘锁的可靠性保证；强制接管他人的 Dispatcher 锁。见 [[TASK-2nt8u]]。
- serve 重启后收养仍在运行的孤儿 agent；跨 Project 的"全部恢复"与着陆页中断提示；continue 时由人选 Stage、次数上限与自动重试；中断 Run 的"放弃"按钮。见 [[TASK-pd8bj]]。
- agent 沙箱或按 Run 的权限配置；任何 token / 金额上限；用量折算金额与实时用量；检测 worktree 外写入；过滤继承的环境变量密钥；限制谁写的 ticket 能触发拾取；启动失败阈值可配置。见 [[TASK-kbyh2]]。
- 同仓库多 Project 的 `agent/<TASK-id>` 分支名撞车；人工 git 操作受合并锁约束；merge 排队超时。见 [[TASK-a8246]]。
- Run 记录的保留期与回收；"全部清理"与跨 Project 的遗留列表；清理没有 Run 记录的 worktree / `agent/*` 分支；重新拾取时改名保留旧分支；未持 Dispatcher 锁时的任何清理。见 [[TASK-6hqkh]]。
