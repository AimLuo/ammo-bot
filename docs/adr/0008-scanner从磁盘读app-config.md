# Scanner 从磁盘读 app config

**Armed** 写在本机 **app config** JSON 里，不进 Beads。Board 的 API 与 scanner（instrumentation）会被 Next 打成两份模块；模块级内存缓存会让 scanner 在 Arm 之后仍看见启动时的快照。每次 `getConfig` 读磁盘，不另做 SQLite 或进程内第二份缓存。同一时刻只跑一个 ammo-bot 进程。
