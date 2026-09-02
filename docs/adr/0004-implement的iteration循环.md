# implement 采用 iteration 循环

Ammo-bot 不调用 Sandcastle 的 `run()`，但 implement 不是一次 CLI 打完。它重复 **iteration**（每次新的 `agent --print`，因为 Cursor 不能 resume），直到 **completion signal** 或 100 次——与 `parallel-planner-with-review` 的 implementer 上限相同。每次 **iteration** 最多一个 commit。review 仍是一次 **iteration**。
