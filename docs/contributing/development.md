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

[`tools-reference.md`](../tools-reference.md) is written from the schemas of the tools; a test
fails when it is out of date: `UPDATE_DOCS=1 cargo test tools_reference` writes it again.

## Tests

| Where | What | Needs |
|---|---|---|
| `src/**` | the rules, the scope, the limits, the audit, what the tools return | nothing |
| `tests/cli.rs` | `--version`, `--help`, a missing configuration | nothing |
| `tests/fake.rs` | `passbolt-mcp` itself, against a fake Passbolt: login, administrators refused, `verify`, a server that cannot prove its key, the tools over stdio | nothing |
| `tests/e2e.rs`, `tests/mcp.rs` | everything, against a real Passbolt server: reading, secrets, writing, sharing, the audit file, resources and prompts | a server (`--ignored`) |

The fake Passbolt is the one of the SDK tests (`tests/common`, copied from `passbolt-sdk` at the
version this crate uses, with its test keys in `tests/fixtures`), behind a local HTTP server: the
binary logs in with GPGAuth and decrypts secrets as against a real server.

### Against a real server

As an MCP user (role user, without MFA), with a second user to share with: copy
`.env.e2e.example` to `.env.e2e` and fill it in, then

```bash
scripts/e2e.sh                # every test that needs a server
scripts/e2e.sh --test mcp     # only the MCP client
E2E_THREADS=1 scripts/e2e.sh  # one test at a time
```

The script writes a configuration in a temporary directory, pins the server key, and runs the
ignored tests. The tests that write work in folders of their own (`mcp-e2e-…`) and delete them
at the end.

## Try it with MCP Inspector

[MCP Inspector](https://github.com/modelcontextprotocol/inspector) lists the tools, resources and
prompts the rules offer, and calls them by hand:

```bash
npx @modelcontextprotocol/inspector passbolt-mcp --config passbolt-mcp.toml
```

Over HTTP, start `passbolt-mcp serve --http 127.0.0.1:8808` and connect the inspector to
`http://127.0.0.1:8808/mcp` (transport Streamable HTTP) through its proxy, the default: a direct
connection comes from the browser page, with an `Origin`, and `passbolt-mcp` refuses it.

## Logs

On stderr, filtered by `PASSBOLT_MCP_LOG` (`warn` by default, `debug` to see more): in stdio,
stdout carries the MCP messages and nothing else.
