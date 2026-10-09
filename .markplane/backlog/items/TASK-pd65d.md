---
id: TASK-pd65d
title: 仓库接上游并清理上游 .markplane 数据
status: in-progress
priority: high
type: chore
effort: medium
epic: EPIC-7m4ss
plan: null
depends_on: []
blocks:
- TASK-bzast
related: []
assignee: chuck
tags:
- ready-for-agent
position: a8
created: 2026-10-09
updated: 2026-10-09
---

# 仓库接上游并清理上游 .markplane 数据

## Parent

Markplane 作为 mattpocock skills 的 tracker 后端 [[EPIC-7m4ss]]

## What to build

让本仓库成为 markplane 的 fork：与上游历史建立关联，并清掉上游自带的项目数据，使 `.markplane` 只描述本项目。

- 添加 `upstream` remote，一次性 `git merge -s ours --allow-unrelated-histories 73cce05`，之后上游合并为普通三方合并。
- 删除 `.markplane/` 下上游的全部条目与归档（含 notes 的 `decisions.md`、`ideas.md`），保留 `config.yaml` 与 `templates/`；**不得删除 [[EPIC-7m4ss]] 及其实现工单**。
- `config.yaml` 的 description 改为说明本 fork 的定位。
- 版本号改为 `0.1.4+skills.1`，`markplane --version` 可见该后缀；`release.yml`、Homebrew、`install.sh` 不动。

## Acceptance criteria

- [ ] `git remote -v` 有 `upstream`；`git log` 中可见对 `73cce05` 的 `-s ours` 合并提交
- [ ] `markplane ls`、`ls epics/plans/notes`（含 `--archived`）只剩 EPIC-7m4ss 及其工单
- [ ] `markplane sync` 后 `.context/summary.md` 描述的是本 fork
- [ ] `markplane --version` 输出 `0.1.4+skills.1`
- [ ] `cargo test --workspace`、`cargo clippy --workspace -- -D warnings` 通过

## Blocked by

- None (can start immediately)
