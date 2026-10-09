---
id: TASK-sad8p
title: 如何持续同步上游 markplane 的更新
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-vs3wh
depends_on: []
blocks: []
related: []
assignee: null
tags: []
position: aQ
created: 2026-10-09
updated: 2026-10-09
---

# 如何持续同步上游 markplane 的更新

## Question

本 fork 如何跟进上游 markplane（`zerowand01/markplane`）的后续提交：是否添加 `upstream` remote 并定期 merge / rebase，还是一次性导入后不再同步？版本号、CHANGELOG、release workflow、Homebrew / install.sh 等发布渠道如何处理（沿用、停用还是改指向本仓库）？

## Answer

2026-10-09 与用户 grilling 确认（全部采纳推荐）：

1. **建立历史关联**：添加 `upstream` remote（`https://github.com/zerowand01/markplane`），执行一次 `git merge -s ours --allow-unrelated-histories 73cce05`——不改任何文件，只把上游 `73cce05` 记为祖先；此后 `git merge upstream/master` 即以其为基的正常三方合并。（背景：源码经 `git archive` 导入，与上游无共同历史。）
2. **节奏与冲突规则**：不定期、按需手动同步。`.markplane/` 条目冲突一律取删除；我们新增功能与上游改动冲突时以我们为准，再手动吸收上游修复；每次同步后 `cargo test`、`clippy`、UI 构建全部通过才提交。
3. **版本号**：用 build metadata 标识 fork，如 `0.1.4+skills.1`（只改 workspace `Cargo.toml` 一行）。
4. **发布渠道**：`release.yml`、Homebrew 相关、`install.sh` 原样保留不动（remote 为裸仓库，不会运行，也减少合并冲突）；本 fork 只支持从源码 `cargo install --path crates/markplane-cli --features embed-ui` 安装，README 安装一节由“README、docs 与 CLAUDE.md 的改写范围”改写。
