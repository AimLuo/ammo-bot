# human flag 暂停 scanner 与 pipeline

**Needs You** 的进件标记沿用 Scotty：Beads 的 `human` 标签（**human flag**），外加就绪的 **human-approval gate**。不是 `bd human` 命令，不是 **Origin**，merge 受阻也不新建 gate。

pipeline 在 merge 合不进 **primary checkout** 时打上这张标签（ADR-0001）；谁打的都算。agent 在 **iteration** 中发现必须等人时也自行打上并 comment。**human flag** 一出现就杀掉当前 Cursor 进程——不等这轮 `agent --print` 自己结束；pipeline 进程不退出。scanner 不领带 **human flag** 的 bead。摘掉且不 close 才 **Resume**，下一轮 prompt 带上这张 **bead** 的 comments。不用 gate、不在 dirty 时另开 checkout、不等待期间退出 pipeline 交给 scanner 再领。
