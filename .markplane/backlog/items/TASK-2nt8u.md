---
id: TASK-2nt8u
title: 强制单 Dispatcher
status: done
priority: medium
type: grilling
effort: medium
epic: null
plan: null
map: MAP-8aavc
depends_on: []
blocks: []
related: []
assignee: chuck
tags: []
position: aq
created: 2026-10-10
updated: 2026-10-10
---

# 强制单 Dispatcher

## Question

是否以及如何强制"同一仓库只有一个 Dispatcher"？

[[TASK-k6d9y]] 默认同一仓库只有一个 Dispatcher（拾取状态不提交、不跨机器共享）。现有 `serve.pid` 锁只管单个用户配置目录：两个 serve 用不同 `XDG_CONFIG_HOME` 指向同一项目，或两台机器共享同一目录（网络盘），就会重复拾取同一 ticket。是否在项目内（如 `.markplane/.runs/` 下）加锁；锁冲突时第二个 serve 是拒绝开扫描开关还是只读；多台机器各自 clone 的情况是否明确不管。

## Answer

## Answer

### 结论

要强制。用 **仓库级 Dispatcher 锁**：一把 OS 建议锁（advisory lock），放在仓库的 git common dir 下，按"仓库 + Project 在仓库中的相对路径"区分。持锁的 serve 才是这个 Project 的 Dispatcher；其他 serve 照常看板、改 ticket，但不能开扫描开关、不能启动或恢复任何 Stage。锁随进程退出由内核自动释放，不需要做过期检测。另一台机器上的独立 clone 明确不管。

### 已核实的代码事实

| 事实 | 位置 | 对本题的意义 |
| --- | --- | --- |
| `serve.pid` 在用户配置目录，进程整个生命周期用 std `File::try_lock` 持排他锁；`lock_held` 用 `try_lock_shared` 探测；内容是 pid / port / url / version / startedAt | `serve_daemon.rs` `PidLock` | 只能阻止同一配置目录起第二个 serve；换 `XDG_CONFIG_HOME` 就能绕过 |
| `.markplane/.locks/<ID>.lock` 是按 item 的 sidecar 排他锁（`fs2::lock_exclusive`，会阻塞），目录里自写内容为 `*` 的 `.gitignore`；锁文件故意不回收，避免删文件和加锁之间的竞态 | `project.rs` `lock_item` | 可以照搬的模式：锁 sidecar 而不是锁内容文件（Windows 上被锁的文件不能被替换），也不回收锁文件 |
| 注册表按 realpath 去重，symlink 别名复用同一条目（有测试）；同名目录得到 `-xxxxx` 后缀，属于不同条目 | `serve_registry.rs` `register` | 同一 serve 里，同一个目录不会注册出两个 Project；但同一仓库的**不同 git worktree** 路径不同，会成为两个 Project |
| 本仓现有 4 个手工 git worktree（`.worktrees/prototype-*`、`research-*`），每个都带自己的 `.markplane/` | `git worktree list` | 同一仓库在同一个 serve 里出现成多个 Project 是真实场景，不是假想 |
| 已有依赖：`fs2 0.4`（core 和 cli），Rust std 的 `File::try_lock`（Unix 上是 flock，按打开文件描述，同一进程开两次也会冲突；Windows 上是 LockFileEx）；cli 在 unix 上依赖 `libc` | `Cargo.toml` | 不需要新的锁 crate |
| 代码里还没有 `git rev-parse` 调用，也没有 `.runs` | 全仓 grep | 锁的键要新算 |

### 锁的设计

| 项 | 决定 |
| --- | --- |
| 粒度 | 一个**仓库里的一个 Project** 一把锁。键 = git common dir 的绝对路径 + Project 目录相对所在 worktree 顶层的路径。所以同一仓库的所有 worktree（含 Dispatcher 自己建的 `.worktrees/<TASK-id>`）里的同一个 Project 共用一把锁；monorepo 里不同子目录下的 Project 各用一把，互不阻塞 |
| 位置 | `<git common dir>/markplane/dispatcher/` 下按键命名的 `.lock` 文件，旁边放一个同名 `.json` 记持有者信息。`.git/` 不会被提交，又天然被所有 worktree 共享，正好就是"同一仓库"的范围。不放 `.markplane/.runs/`，因为那是按工作副本分开的，挡不住 worktree 的情况 |
| 机制 | 用非阻塞 `try_lock` 加 OS 建议锁，跟 `serve.pid` 一样。锁文件保持为空、不回收（理由同 `.locks/`）；持有者信息写进旁边的 `.json`（原子写），不写进被锁的文件，避免 Windows 上被锁区域读不了 |
| 持有者信息 | pid、主机名、serve URL、用户配置目录、Project 路径、取得锁的时间。**只用来显示**；锁是否有效只看 OS 锁有没有被持有，不靠 PID 或主机名判断 |
| 过期 | 不做过期检测：进程崩溃或被杀，内核自动释放。锁空闲时 `.json` 里的旧信息一律忽略，下次取锁时覆盖 |
| 何时取 | 打开扫描开关时取锁；"恢复执行"、comment and continue 等任何要启动 Stage 的动作，如果当前没持锁也要先取锁 |
| 持有多久 | 只要扫描开关开着，**或者**这个 Project 在本 serve 里还有活动的 Run（`running` / `merge-queued`），就一直持有。关掉扫描后，等活动 Run 全部结束才释放；serve 退出时自动释放 |
| 持锁才能做的事 | 拾取 ticket、启动任何 Stage、Dispatcher 对 ticket 状态 / 标签 / assignee 的写入、写 `.runs/`、Dispatcher 发起的 worktree 和 `agent/*` 分支增删。人和普通 CLI / MCP 的编辑不受影响（仍走 `.locks/`） |
| 防锁被删 | 每次扫描（15 秒）核对锁文件路径和持有的句柄是否还是同一个文件；不一致就当作锁已丢失：关闭扫描开关、不再拾取，正在跑的 Stage 跑完当前这一步，`serve.log` 记警告，界面提示 |
| 同一 serve 内重复 | serve 在内存里按锁键记账：同一个键已经被本 serve 的另一个 Project 条目持有时，直接拒绝，并指出是哪个 Project（例如两个 worktree 都注册了）。OS 锁本身也会冲突，内存记账只是为了给出更清楚的提示 |

### 冲突时的行为

- 第二个 serve **只读，不降级成"半个 Dispatcher"**：看板、编辑、评论照常；打开扫描开关、恢复执行、continue 一律被拒，返回 409 `dispatcher_locked`，带上持有者信息。扫描开关保持关闭。
- 提示文案要能直接照着处理，例如："此仓库的 Dispatcher 已由 <主机名> 上 pid <pid>（<URL>，Project <路径>）持有，始于 <时间>。请先关掉那边的扫描开关或停掉那个 serve。" 主机名和本机不同时，额外说明"在另一台机器上"。
- 不提供"强制接管"：建议锁抢不过来，强行做只会制造两个写入者。
- 没持锁的 serve **不能**把 `.runs/` 里非终态的 Run 当作"被中断"：先试着取锁，取不到就说明这些 Run 正在别处运行，只能只读显示。
- 每个 Project 的 Dispatcher 状态里要能读到锁的情况（空闲 / 本 serve 持有 / 被他人持有及持有者），用不改变状态的共享锁探测，跟 `lock_held` 一样。

### 覆盖范围

| 场景 | 是否覆盖 | 说明 |
| --- | --- | --- |
| 同一配置目录起第二个 serve | 已有 | `serve.pid` |
| 不同 `XDG_CONFIG_HOME` 指向同一个目录 | 覆盖 | 仓库锁 |
| 同一仓库的多个 git worktree 注册成不同 Project（同一个 serve 或不同 serve） | 覆盖 | 键用 git common dir |
| symlink 或别名路径 | 覆盖 | 注册表去重，键也用 realpath |
| 同一 Project 在一个 serve 里出现两次 | 注册表层面不会出现；即便出现（如 bind mount）也会被锁挡住 | 内存记账加 OS 锁 |
| 网络盘上的共享目录被两台机器同时用 | 尽力而为 | 取决于文件系统是否真的支持建议锁；不承诺 |
| 同机或异机的独立 clone（各自有 `.git`） | **明确不管** | 拾取状态不提交（[[TASK-k6d9y]]），没有共享介质；文档写明"一个仓库只在一个地方开扫描" |
| 不是 git 仓库 | 不涉及 | 打开扫描前本来就要求是 git 仓库（[[TASK-cq846]]） |

### 自问自答（均按推荐采纳）

| # | 问题 | 采纳的答案 |
| --- | --- | --- |
| 1 | 要不要在 `serve.pid` 之外再强制单 Dispatcher？ | 要；`serve.pid` 只管一个配置目录 |
| 2 | 锁的粒度按工作副本（`.markplane/`）还是按仓库？ | 按仓库里的 Project：git common dir 加 Project 的相对路径 |
| 3 | 锁放哪？ | `<git common dir>/markplane/dispatcher/`，不放 `.markplane/.runs/` |
| 4 | 用 OS 建议锁还是 PID 文件加存活检测？ | OS 建议锁，非阻塞 `try_lock`；PID 和主机名只用于显示 |
| 5 | 崩溃后要不要做过期检测？ | 不做；内核自动释放，旧的持有者信息忽略 |
| 6 | 何时取锁？ | 打开扫描开关时；或者在没持锁时启动任何 Stage（恢复执行、continue）之前 |
| 7 | 何时释放？ | 扫描开关关闭且没有活动 Run 时；或 serve 退出 |
| 8 | serve 启动时就锁上所有注册的 Project 吗？ | 不；没开扫描的 Project 不应挡住别人 |
| 9 | 冲突时第二个 serve 怎么办？ | 只读；拒绝开扫描、恢复、continue，返回 409，附持有者信息；不提供强制接管 |
| 10 | 没持锁的 serve 怎么对待非终态的 Run 记录？ | 不认定为中断；取不到锁就只读显示"在别处运行中" |
| 11 | 同一 serve 里同一仓库出现两次（多个 worktree）？ | 按锁键在内存里记账，拒绝并指出冲突的 Project |
| 12 | 锁文件被人删了怎么办？ | 每次扫描核对是否还是同一个文件；不一致就关扫描、停止拾取、告警 |
| 13 | 网络盘？ | 尽力而为，不承诺 |
| 14 | 独立 clone（同机或异机）？ | 明确不管，写进文档 |
| 15 | 持有者信息写进锁文件本身吗？ | 不；写进旁边的 `.json`，锁文件保持为空、不回收 |
| 16 | 人和普通 CLI 的编辑要不要也受这把锁限制？ | 不要；只限制 Dispatcher 发起的动作 |

### 依赖与假设

- 修正 [[TASK-2vpdd]] 里"`serve.pid` 保证单写入者"的前提：保证 `.runs/` 只有一个写入者的是本票的仓库锁；进程内每个 ticket 一把互斥锁的做法不变。
- [[TASK-exz6h]]（并行进行中）：假设界面能显示"锁被他人持有"的状态和持有者信息；具体怎么呈现由该票决定。
