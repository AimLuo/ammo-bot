---
id: TASK-kcivc
title: HTTPS 与自签名证书
status: done
priority: medium
type: feature
effort: medium
epic: EPIC-rm3zt
plan: null
depends_on:
- TASK-d5syu
blocks:
- TASK-kfzkx
related: []
assignee: chuck
tags:
- ready-for-agent
position: aZ
created: 2026-10-10
updated: 2026-10-10
---

# HTTPS 与自签名证书

## Parent

markplane serve：多 Project 切换、后台运行与 HTTPS [[EPIC-rm3zt]]

## What to build

正式模式只开 HTTPS，做法见 Epic 中“HTTPS”。

- 用 `axum-server`（`tls-rustls`）提供 HTTPS 与 wss；全程只用 aws-lc-rs，rcgen 关闭默认特性。
- 首次启动在用户配置目录 `certs/` 下生成 ECDSA P-256 自签叶子证书：SAN `localhost` / `127.0.0.1` / `::1`，EKU serverAuth，KeyUsage digitalSignature（critical），BasicConstraints `CA:FALSE`（critical），有效期约 800 天，随机序列号。目录 0700、`key.pem` 0600、`cert.pem` 0644。证书缺失、剩余不到 30 天或 SAN 不符时重签并提示重新信任。
- 生成或重签后打印当前 OS 的信任命令（macOS 钥匙串；Linux Chrome NSS `"P,,"`、Firefox 各 profile `"C,,"`）。
- `--dev` 保持 HTTP；`--no-tls` 回退 HTTP。
- CORS 白名单、WebSocket Origin 校验与打印的 URL 按实际协议生成。
- CI 增加一次 `x86_64-unknown-linux-musl` 构建验证。

## Acceptance criteria

- [ ] 默认 `markplane serve` 以 `https://localhost:<port>` 提供页面、API 与 wss
- [ ] 证书字段、有效期与文件权限符合上述要求（测试读取生成的证书校验）
- [ ] 删除证书或改动 SAN 后重启会重签并再次打印信任命令；未到期且匹配时复用
- [ ] `--dev` 与 `--no-tls` 走 HTTP，CORS / Origin 校验随之调整
- [ ] CLI 集成测试完成一次信任生成证书的 HTTPS 握手
- [ ] CI 的 musl 构建通过
