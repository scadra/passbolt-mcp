---
title: Troubleshooting
sidebar_position: 9
---

# Troubleshooting

`passbolt-mcp check` runs everything the server does at start, then stops: start there. More
detail with `PASSBOLT_MCP_LOG=debug`, on stderr.

## Configuration

| Message | What to do |
|---|---|
| `cannot read …/passbolt-mcp.toml` | No file at that path. Give `--config` an absolute path: MCP clients do not expand `~`. |
| `invalid …/passbolt-mcp.toml` | The TOML does not parse, or a key is unknown; the rest of the message says which. |
| `no passphrase: set passphrase_file or passphrase_command` | One of the two, not both. |
| `cannot read the passphrase from …`, `the passphrase is empty` | The first line of the file is the passphrase. |
| `cannot run …`, `… failed` | `passphrase_command` did not run, or failed: run it yourself. It gets no stdin, so it cannot ask a question. |
| `cannot read the private key …` | Relative paths start at the directory of the configuration file. |

## Login

| Message | What to do |
|---|---|
| `the server is not verified (… is missing): run passbolt-mcp verify` | Pin the server key once: `passbolt-mcp verify`. |
| `the server key changed: … is pinned, the server sends …` | Either the server changed its key, or another server answers at that address. Ask the Passbolt administrator before deleting the pinned key and running `verify` again. |
| `Server identity mismatch` | The server presents the pinned key without proving it holds it: do not go on. |
| `cannot log in as the MCP user` | A wrong passphrase, a key the server does not know, or a user not activated or suspended. The rest of the message comes from Passbolt. |
| `… is an administrator: the MCP user must have the role user` | Create a user of its own for the agent: see [the MCP user](guides/mcp-user.md). |
| MFA asked | Set `totp_secret_file`. Other MFA providers (Yubikey, Duo) cannot be used by an agent. |
| A timeout | A large server takes long to list everything: raise `timeout_seconds`. |
| A certificate error | A server with a private certificate authority: add it to the system. `tls_skip_verify` is for a test server only. |

## The client

| Symptom | What to do |
|---|---|
| The server does not show, or fails at start | Run the exact command of the client configuration in a terminal: the error is on stderr. Most clients show that stderr in their MCP logs too. |
| A tool is missing | The rules do not offer it: `whoami` says what they allow, and `check` which key each needs. A tool refused by the rules is never listed. |
| `refused by the MCP rules: access.… is false` | The tool needs that key. |
| `refused by the MCP rules: outside scope.folders` | The resource or folder is outside `scope.folders`. |
| `refused by the MCP rules: over limits.…` | Wait a minute, or raise the limit. |
| `forbidden` | Passbolt does not give the MCP user the permission needed: share it as **Update** to change it, **Owner** to share it. |
| `not_found` | Not shared with the MCP user, or deleted. |
| With Docker, nothing happens | `-i` is missing, or a `-t` is there. |
| Over HTTP, `403` | A request from a web page, with an `Origin`: `passbolt-mcp` refuses them. Connect from the client, or from MCP Inspector through its proxy. |

## Still stuck

Check the tools by hand with [MCP Inspector](contributing/development.md#try-it-with-mcp-inspector),
then ask on the [community forum](https://community.passbolt.com) or open an
[issue](https://github.com/scadra/passbolt-mcp/issues), with the output of `passbolt-mcp check`.
Never paste a key, a passphrase or a secret.
