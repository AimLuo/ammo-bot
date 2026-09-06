# 合入后同步 remote

Merger 在合入 **target branch** 之后，按目标项目 AGENTS.md 的单仓/多仓把提交推到存在的 git remote：有名为 `origin` 的 remote 就推父仓；多仓则先推本次改动的 `repos/<名>` subtree。不看 AGENTS.md 的「不 push / 父仓本地」策略行。禁止 force；缺 subtree URL 或非快进则 **human flag**。pipeline 独占 close：**completion signal** 之后若 `origin` 仍领先，当 **merger** 未完成。

脚本 merge 被否掉（ADR-0005）；把 push 做成独立 Publish 步骤也被否掉——同一把 **merge lock**、同一次 **iteration**。
