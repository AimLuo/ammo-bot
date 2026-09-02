# 合入 primary checkout

Pipeline 在 linked **worktree** 里做 implement 和 review，再把结果合进该 **project** 的 **primary checkout**（登记 path）上的 **target branch**。单独再开一个 merge worktree 被否掉：常见情况下 primary checkout 是干净的；脏或合不进去就进 **Needs You**，而不是第二份 checkout。

若 primary checkout 接受不了 merge（有未提交文件，或其它 git 拒绝），这张 bead 打上 **human flag**，保持 `in_progress`——scanner 不会再从 **ready** 领它。
