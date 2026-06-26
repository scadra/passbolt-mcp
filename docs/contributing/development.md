---
title: Development
sidebar_position: 1
---

# Development

Rust 1.88 or later; no OpenSSL. The Passbolt API and OpenPGP come from
[`switch-case-passbolt-sdk`](https://crates.io/crates/switch-case-passbolt-sdk), imported as
`passbolt_sdk`.

```bash
cargo build
cargo test
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
bash scripts/license-headers.sh          # --fix adds the header to new files
cargo deny check                         # advisories, licenses, sources
```

The CI runs the same, plus `cargo check --locked` with Rust 1.88.

Against a Passbolt server, with the configuration of an MCP user and after `passbolt-mcp verify`,
the login and an MCP client calling `passbolt-mcp` over stdio:

```bash
PASSBOLT_MCP_E2E_CONFIG=passbolt-mcp.toml PASSBOLT_MCP_E2E_SHARE_WITH=betty@example.com \
  cargo test -- --ignored
```

`PASSBOLT_MCP_E2E_SHARE_WITH` is another user of the server, to share with. The tests that write
work in folders of their own (`mcp-e2e-…`) and delete them at the end; the configuration must
have no `[access]` section, which they add.

Logs go to stderr, filtered by `PASSBOLT_MCP_LOG` (`warn` by default, `debug` to see more): in
stdio, stdout carries the MCP messages and nothing else.
