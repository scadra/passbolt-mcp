---
title: Installation
---

# Installation

passbolt-mcp is a single binary, with nothing else to install: no OpenSSL and no runtime. Install
it on the machine where the agent runs, because the MCP client starts it there.

You can install it with Homebrew, from a release, as a Claude Desktop extension, with Docker, or
from crates.io.

## With Homebrew

On macOS and Linux:

```bash
brew install scadra/tap/passbolt-mcp
```

`brew upgrade passbolt-mcp` installs the next releases.

## From a release

Each [release](https://github.com/scadra/passbolt-mcp/releases) holds one archive per platform,
named `passbolt-mcp-v<version>-<target>`, and its SHA-256 next to it:

| System | Processor | Target | Archive |
|---|---|---|---|
| Linux, any distribution | x86_64 | `x86_64-unknown-linux-musl` | `.tar.gz` |
| Linux, any distribution | ARM64 | `aarch64-unknown-linux-musl` | `.tar.gz` |
| macOS | Apple silicon | `aarch64-apple-darwin` | `.tar.gz` |
| macOS | Intel | `x86_64-apple-darwin` | `.tar.gz` |
| Windows | x86_64 | `x86_64-pc-windows-msvc` | `.zip` |

To find your system and processor, run `uname -sm`: it prints, for example, `Linux x86_64` or
`Darwin arm64`. The Linux binaries are static: they run on any distribution, Alpine included. Then,
on Linux and macOS:

```bash
VERSION=0.1.0
TARGET=x86_64-unknown-linux-musl
NAME=passbolt-mcp-v$VERSION-$TARGET
BASE=https://github.com/scadra/passbolt-mcp/releases/download/v$VERSION
curl -fLO "$BASE/$NAME.tar.gz"
curl -fLO "$BASE/$NAME.tar.gz.sha256"
shasum -a 256 --check "$NAME.tar.gz.sha256"   # NAME.tar.gz: OK
tar xzf "$NAME.tar.gz"
install -m 755 "$NAME/passbolt-mcp" ~/.local/bin/passbolt-mcp
```

If the check prints anything other than `OK`, delete the archive and download it again.

On Windows, extract the `.zip` and put `passbolt-mcp.exe` in a folder of your `PATH`.

## As a Claude Desktop extension

Each release also holds `passbolt-mcp-v<version>.mcpb`, a
[desktop extension](https://www.anthropic.com/engineering/desktop-extensions) for Claude Desktop on
macOS and Windows, with the binary inside:

1. Write the configuration first: follow the [quick start](quick-start.md) up to `passbolt-mcp
   check`, with a binary from a release.
2. Open the `.mcpb` file: Claude Desktop shows what passbolt-mcp is and the tools it offers.
3. Choose the configuration file, `passbolt-mcp.toml`, and install.

Claude Desktop then starts passbolt-mcp itself, and updates it when you open the bundle of a newer
release.

## With Docker

```bash
docker pull ghcr.io/scadra/passbolt-mcp:0.1.0
```

The image is available for Linux on amd64 and arm64. It serves over stdio and reads its
configuration from `/config`. See [Connect a client with Docker](../guides/serve.md#with-docker).

## From crates.io

You need Rust 1.88 or later:

```bash
cargo install --locked passbolt-mcp
```

From the sources of a tag instead:
`cargo install --locked --git https://github.com/scadra/passbolt-mcp --tag v0.1.0`.

## Check the installation

```bash
passbolt-mcp --version
```

## Next steps

Follow the [quick start](quick-start.md) to connect your first agent.
