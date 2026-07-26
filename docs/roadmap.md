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
| **The tools** | 28 tools: reading, secrets (`get_secret`, `get_totp_code`, `check_password_pwned`, `generate_password`, `run_with_secret`), writing, sharing; MCP resources and prompts. See the [tools reference](tools-reference.md). |
| **The password policy** | `generate_password` follows the password policy of the organization: password or passphrase, length, character sets; PIN codes; the strength said, and whether it is below the policy. |
| **Secrets without seeing them** | `access.mask_secrets`: handles in place of values; `run_with_secret` runs a command of `[commands]` with the password in its environment, its output scrubbed. See [Use a secret without seeing it](secrets-without-seeing.md). |
| **Transports** | stdio; Streamable HTTP on loopback, which refuses browser pages (`Origin`). |

### Tested against a real Passbolt

Every feature runs against a real Passbolt server (ddev, `scripts/e2e.sh`), through a real MCP
client that starts the `passbolt-mcp` binary:

| Area | Tests |
|---|---|
| Login | the MCP user logs in with GPGAuth, the server proves its pinned key, the session is renewed when it ends; `passbolt-mcp verify` and `check` |
| Reading | resources, folders, folder tree, resource types, comments, permissions, search, favorites; MCP resources and prompts |
| Secrets | `get_secret` (password and note), `get_totp_code` (never the seed), `check_password_pwned`, `generate_password` (the policy of the server, passphrases, PIN codes, strength); a secret read with every log at `trace` is in none of them; with `mask_secrets`, handles, `run_with_secret` with a handle, a resource id or a fixed resource, the output scrubbed, a password rotated through a handle |
| Writing | resources (name, username, URIs, description, note, expiry), folders (create, rename, move, delete with or without content), comments, favorites, `dry_run` |
| Sharing | `search_aros`, `simulate_share`, sharing and revoking resources and folders, moves that give a folder's permissions only with `access.share` |
| Rules | off by default; `allow_tools`, `deny_tools`; `scope.folders` on reads and writes; `limits`; the audit file |
| Confirm | accepted, declined, cancelled; protocol 2025-11-25 (`elicitation/create`) and 2026-07-28 (`input_required`); a client that cannot ask |
| HTTP | a client over Streamable HTTP; a request from a browser page refused |

The unit tests and a fake Passbolt cover the rest offline, on each pull request.

## Next

| | What | Why |
|---|---|---|
| **1** | [Secrets the agent uses without seeing them](secrets-without-seeing.md#what-comes-next), the rest | Handles and `run_with_secret` are there. Templates, the HTTP proxy, and showing a secret to the person only are next. |
| **2** | Distribution | crates.io, a Claude Desktop bundle (`.mcpb`), install buttons for Cursor and VS Code, a static Linux binary, Homebrew. |
| **3** | HTTP with a token | Streamable HTTP beyond loopback, behind `Authorization: Bearer`, for a server shared by agents; Docker with a token. |
| **4** | Profiles | One `passbolt-mcp` serving several agents, each with its MCP user and its rules (`--profile`). |
| **5** | Clients checked before each release | Claude Code, Claude Desktop, Cursor and VS Code, by hand: the configurations of [Connect a client](guides/serve.md), and the question of `[confirm]` as each client shows it. The protocol itself is already tested on every change, through the MCP client of the Rust SDK. |

Later, in the SDK first: `passbolt-mcp init <invitation link>`, which generates the key of the
MCP user and activates it without a browser.
