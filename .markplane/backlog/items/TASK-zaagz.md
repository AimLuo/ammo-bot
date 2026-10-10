---
id: TASK-zaagz
title: axum 怎么接 rustls 提供 HTTPS/wss，并用 rcgen 生成浏览器可接受的自签名证书
status: draft
priority: medium
type: research
effort: medium
epic: null
plan: null
map: MAP-3fjjb
depends_on: []
blocks: []
related: []
assignee: null
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
