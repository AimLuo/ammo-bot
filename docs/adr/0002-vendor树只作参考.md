# vendor 树只作参考

Ammo-bot 从 bead-me-up-scotty 派生，流水线形状参考 Sandcastle，但不改 `repos/bead-me-up-scotty` 和 `repos/sandcastle`。活代码在 ammo-bot。也包括用 `git worktree add` 创建 `.worktrees/`——不 fork Sandcastle 的 WorktreeManager 去改它写死的 `.sandcastle/worktrees/`。
