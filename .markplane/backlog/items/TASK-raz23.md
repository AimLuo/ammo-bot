---
id: TASK-raz23
title: 把 markplane 拷入仓库根目录并跑通构建和测试
status: done
priority: medium
type: task
effort: medium
epic: null
plan: null
map: MAP-vs3wh
depends_on: []
blocks:
- TASK-w5rd5
related: []
assignee: null
tags: []
position: aH
created: 2026-10-09
updated: 2026-10-09
---

# 把 markplane 拷入仓库根目录并跑通构建和测试

## Question

将 `.tmp/markplane` 的源码（不含其 `.git`、`target`、`node_modules`）拷入仓库根目录，使仓库结构与 markplane 一致；跑通 `cargo build --workspace`、`cargo test --workspace`、`cargo clippy --workspace` 与 Web UI 构建。记录：所用的上游 commit、构建/测试结果、任何需要的环境前提（Rust / Node 版本）。

此工单只做“拷入并跑通”，不改任何行为；它解锁 MAP 页面原型与后续对照代码的讨论。

## Answer

2026-10-09 完成（AFK）：

- **来源**：上游 markplane commit `73cce05e5f8129e8d0bb892b9be895c5065484bb`（“docs: include Cargo.lock in the release version bump (#79)”），以 `git -C .tmp/markplane archive HEAD | tar -x` 导出全部受版本控制文件到仓库根目录（含 `.claude/`、`.github/`、`.markplane/`、`assets/`、`crates/`、`docs/` 等）。
- **覆盖**：根目录原 `README.md` 被上游版本覆盖；`.gitignore` 换为上游版本，并追加 `.tmp/`、`.worktrees/`。
- **环境**：rustc / cargo 1.93.0（满足 `rust-version = "1.93.0"`），Node v24.20.0，npm 11.19.0。
- **结果**：
  - `cargo build --workspace` 通过；
  - `cargo test --workspace` 全绿（6 + 76 + 70 + 294 个测试，0 失败）；
  - `cargo clippy --workspace` 0 告警；
  - `crates/markplane-web/ui`：`npm ci && npm run build` 通过（npm 提示 `unrs-resolver` 的 install script 未获批准，不影响构建）；
  - `cargo build -p markplane-cli --features embed-ui` 通过，`markplane --version` 为 `0.1.4`，`markplane dashboard` 正常读取。
- **注意**：上游自身的 `.markplane/` 项目数据（markplane 的 backlog / roadmap，26 个开放条目）随之拷入，去留尚未决定（已记入 map 的 Not yet specified）。
- 改动尚未提交（工作区未 commit）。
