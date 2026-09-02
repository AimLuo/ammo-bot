# merge lock 下的 merger

无冲突时用脚本 `git merge` 被否掉，那是砍掉了 Sandcastle 的 merger。implement 和 review 之后，ammo-bot 拿 **merge lock**，在 **primary checkout** 上跑 Cursor **merger**：合入这一条 named branch、解冲突、typecheck 和测试、close 这张 **bead**、发出 **completion signal**。一次 **iteration**（与模板 merger 上限相同）。其它 pipeline 可以并行 implement；只有 merge 才等这把锁。
