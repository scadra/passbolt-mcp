---
title: Installation
sidebar_position: 1
---

# Installation

`passbolt-mcp` is a single binary, with nothing else to install: no OpenSSL, no runtime. Install it
on the machine where the agent runs: the MCP client starts it.

## From a release

Each [release](https://github.com/scadra/passbolt-mcp/releases) holds one archive per platform,
named `passbolt-mcp-v<version>-<target>`, and its SHA-256 next to it:

| System | Processor | Target | Archive |
|---|---|---|---|
| Linux (glibc) | x86_64 | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Linux (glibc) | ARM64 | `aarch64-unknown-linux-gnu` | `.tar.gz` |
| macOS | Apple silicon | `aarch64-apple-darwin` | `.tar.gz` |
| macOS | Intel | `x86_64-apple-darwin` | `.tar.gz` |
| Windows | x86_64 | `x86_64-pc-windows-msvc` | `.zip` |

`uname -sm` tells your system and processor (`Linux x86_64`, `Darwin arm64`…). On Linux and
macOS:

```bash
VERSION=0.1.0
TARGET=x86_64-unknown-linux-gnu
NAME=passbolt-mcp-v$VERSION-$TARGET
BASE=https://github.com/scadra/passbolt-mcp/releases/download/v$VERSION
curl -fLO "$BASE/$NAME.tar.gz"
curl -fLO "$BASE/$NAME.tar.gz.sha256"
shasum -a 256 --check "$NAME.tar.gz.sha256"   # NAME.tar.gz: OK
tar xzf "$NAME.tar.gz"
install -m 755 "$NAME/passbolt-mcp" ~/.local/bin/passbolt-mcp
```

Anything else than `OK`: delete the archive and download it again. On Windows, extract the `.zip`
and put `passbolt-mcp.exe` in a folder of your `PATH`.

## With Docker

```bash
docker pull ghcr.io/scadra/passbolt-mcp:0.1.0
```

For Linux, amd64 and arm64. It serves over stdio, with its configuration in `/config`: see
[Docker](../guides/serve.md#with-docker).

## From source

With Rust 1.88 or later:

```bash
cargo install --locked --git https://github.com/scadra/passbolt-mcp --tag v0.1.0
```

## Check it

```bash
passbolt-mcp --version
```

Next: [quick start](quick-start.md).
