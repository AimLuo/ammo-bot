# 直接调用 Cursor CLI

Ammo-bot 抄 Sandcastle 的 prompt / pipeline 形状，但不依赖 `@ai-hero/sandcastle`，也不调用 `run()`。implement、review 和冲突处理在 host 上 spawn `agent --print`。创建 worktree、Beads 的 claim/close、merge、**Resume** 都留在 ammo-bot。vendor 的 Sandcastle 树仍只作参考（ADR-0002）。
