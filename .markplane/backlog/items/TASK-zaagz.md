---
id: TASK-zaagz
title: axum 怎么接 rustls 提供 HTTPS/wss，并用 rcgen 生成浏览器可接受的自签名证书
status: done
priority: medium
type: research
effort: medium
epic: null
plan: null
map: MAP-3fjjb
depends_on: []
blocks: []
related: []
assignee: chuck
tags: []
position: aT
created: 2026-10-10
updated: 2026-10-10
---

# axum 怎么接 rustls 提供 HTTPS/wss，并用 rcgen 生成浏览器可接受的自签名证书

## Question

在现有 axum 服务上提供 HTTPS 与 wss，并用 `rcgen` 生成浏览器可接受的自签名证书，应怎么做？需要回答：

- 选 `axum-server`（rustls）还是 `tokio-rustls` 手接；与当前 axum 版本、`--features embed-ui`、musl 静态编译的兼容性。
- Chrome / Safari / Firefox 接受一张被用户信任的自签名叶子证书所需的字段：SAN（`localhost`、`127.0.0.1`）、EKU、BasicConstraints、有效期上限。
- macOS（`security add-trusted-cert`）与 Linux（各发行版 / NSS）信任该证书的命令；证书更新后是否需要重新信任。
- 证书与私钥文件的权限、存放与轮换建议。

## Answer

调研材料：分支 `research/serve-https`（`35ba811`，仅本地），文件 `research/serve-https-self-signed.md`（含一手出处）。本机用原型实测过 HTTPS 与 wss 握手；musl 静态编译未实测。

推荐做法：

- **选型**：`axum-server` 0.8 的 `tls-rustls` 特性，不手接 `tokio-rustls`。与 axum 0.8.9 兼容，自带 WebSocket 升级、优雅停机与证书热重载，现有 `/ws` 路由不用改。
- **加密库**：只保留 aws-lc-rs 一套；rcgen 配成 `default-features = false, features = ["aws_lc_rs", "pem"]`，否则会同时编进 ring。musl 下 aws-lc-rs 只需要 C 编译器（release 流程已装 `musl-tools`）；需在 CI 实测一次 musl 构建。
- **证书字段**（ECDSA P-256）：
  - SAN：`localhost`、`127.0.0.1`、`::1`；
  - EKU：serverAuth；
  - KeyUsage：digitalSignature（critical）；
  - BasicConstraints：`CA:FALSE`（critical），Firefox 会拒绝带 CA 标志的服务器证书；
  - 有效期不超过 825 天（Apple 对用户信任证书的上限），取约 800 天；398 天的限制只针对公共 CA 签发的证书；
  - 序列号随机，每次轮换都换新密钥。
  - 不能直接用 `generate_simple_self_signed`：其默认有效期到 4096 年且不带 EKU，Safari 会拒绝。
- **信任命令**：
  - macOS（Safari / Chrome / Firefox 通用）：`security add-trusted-cert -r trustRoot -p ssl -k ~/Library/Keychains/login.keychain-db cert.pem`
  - Linux Chrome：`certutil -d sql:$HOME/.local/share/pki/nssdb -A -t "P,," -n markplane-localhost -i cert.pem`（只标 `C` 会被 Chrome 拒绝）
  - Linux Firefox：对每个 profile 执行同样的命令，信任参数用 `"C,,"`；两边都用 `"CP,,"` 也行。
  - `update-ca-certificates` 对浏览器不起作用。
  - 换证书后必须重新信任。
- **存放与轮换**：`~/.config/markplane/certs/` 目录权限 0700，`key.pem` 0600，`cert.pem` 0644；只在证书缺失、剩余不到 30 天或 SAN 不符时重签，重签后提示用户重新信任。
- **落地时要一起改**：`serve.rs` 里的 CORS 白名单、WebSocket 的 Origin 校验与打印的 URL 都写死了 `http://`；前端已会按页面协议自动使用 `wss:`。
