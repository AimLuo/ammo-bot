# human flag 暂停 scanner 与 pipeline

**Needs You** 的进件标记沿用 Scotty：Beads 的 `human` 标签（**human flag**），外加就绪的 **human-approval gate**。不是 `bd human` 命令，不是 **Origin**，merge 受阻也不新建 gate。

pipeline 在 merge 合不进 **primary checkout** 时打上这张标签（ADR-0001）；谁打的都算。scanner 不领带 **human flag** 的 bead。进行中的 pipeline 在下一轮 implement / review / **merger** 前若标签还在就进程内等待；摘掉且不 close 才 **Resume**。不用 gate、不在 dirty 时另开 checkout、不等待期间退出 pipeline 交给 scanner 再领。
