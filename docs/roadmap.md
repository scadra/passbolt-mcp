---
title: Roadmap
description: What passbolt-mcp does today, how it is tested, and what comes next.
---

# Roadmap

This page describes what passbolt-mcp does today, how it is tested, and what comes next.

## Today

| | |
|---|---|
| **The MCP user** | A Passbolt user of its own, role user, logged in with its OpenPGP key. It sees what is shared with it, nothing more; administrators are refused. |
| **The rules** | `[access]` (reading, secrets, writing, sharing; `allow_tools`, `deny_tools`), `[scope]` (folders), `[limits]` (calls and secrets per minute), `[audit]` (one JSON line per call, never a secret). |
| **The person in the loop** | `[confirm]`: the person says yes to each call of the tools listed, through the client; the agent cannot answer for them. |
| **The tools** | 31 tools: reading, secrets (`get_secret`, `get_totp_code`, `check_password_pwned`, `generate_password`, `run_with_secret`, `call_api`, `fill_template`, `show_secret`), writing, sharing; MCP resources and prompts. See the [tools reference](tools-reference.md). |
| **The password policy** | `generate_password` follows the password policy of the organization: password or passphrase, length, character sets; PIN codes; the strength said, and whether it is below the policy. |
| **Secrets without seeing them** | By default, handles in place of values; values only with `access.reveal_secrets`. passbolt-mcp puts the secret in a declared command (`run_with_secret`), a request to a declared API (`call_api`) or a declared file (`fill_template`), its output scrubbed, or shows it to the person only (`show_secret`). See [Use a secret without seeing it](secrets-without-seeing.md). |
| **Transports** | stdio; Streamable HTTP on loopback, which refuses browser pages (`Origin`). |
| **Distribution** | Homebrew (`scadra/tap`), crates.io, a Claude Desktop extension (`.mcpb`), install buttons for Cursor and VS Code, static Linux binaries, macOS and Windows binaries, a Docker image for amd64 and arm64. See [Installation](getting-started/installation.md). |

### Tested against a real Passbolt

Every feature runs against a real Passbolt server (ddev, `scripts/e2e.sh`), through a real MCP
client that starts the `passbolt-mcp` binary:

| Area | Tests |
|---|---|
| Login | the MCP user logs in with GPGAuth, the server proves its pinned key, the session is renewed when it ends; `passbolt-mcp verify` and `check` |
| Reading | resources, folders, folder tree, resource types, comments, permissions, search, favorites; MCP resources and prompts |
| Secrets | `get_secret` (password and note), `get_totp_code` (never the seed), `check_password_pwned`, `generate_password` (the policy of the server, passphrases, PIN codes, strength); a secret read with every log at `trace` is in none of them; by default, handles, a TOTP code too, `run_with_secret` with a handle, a resource id or a fixed resource, the output scrubbed, a password rotated through a handle; `call_api` on a local API, paths kept under the base, redirects not followed; `fill_template` with resources, fields and handles, the file readable by its owner only; `show_secret` with both protocols, and a client that cannot show it |
| Writing | resources (name, username, URIs, description, note, expiry), folders (create, rename, move, delete with or without content), comments, favorites, `dry_run` |
| Sharing | `search_aros`, `simulate_share`, sharing and revoking resources and folders, moves that give a folder's permissions only with `access.share` |
| Rules | off by default; `allow_tools`, `deny_tools`; `scope.folders` on reads and writes; `limits`; the audit file |
| Confirm | accepted, declined, cancelled; protocol 2025-11-25 (`elicitation/create`) and 2026-07-28 (`input_required`); a client that cannot ask |
| HTTP | a client over Streamable HTTP; a request from a browser page refused |

The unit tests and a fake Passbolt cover the rest offline, on each pull request.

## Next

| | What | Why |
|---|---|---|
| **1** | HTTP with a token | Streamable HTTP beyond loopback, behind `Authorization: Bearer`, for a server shared by agents; Docker with a token. |
| **2** | Profiles | One `passbolt-mcp` serving several agents, each with its MCP user and its rules (`--profile`). |
| **3** | Clients checked before each release | Claude Code, Claude Desktop, Cursor and VS Code, by hand: the configurations of [Connect a client](guides/serve.md), the Claude Desktop extension, the install buttons, and the question of `[confirm]` as each client shows it. The protocol itself is already tested on every change, through the MCP client of the Rust SDK. |

Later, in the SDK first: `passbolt-mcp init <invitation link>`, which generates the key of the
MCP user and activates it without a browser.
