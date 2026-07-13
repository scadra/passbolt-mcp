---
title: Rules
sidebar_position: 4
---

# Rules

The rules are the configuration of `passbolt-mcp`. Passbolt decides what the
[MCP user](guides/mcp-user.md) can see; the rules decide what the agent can do with it. The agent
gets what both allow.

## The file

`passbolt-mcp` reads `passbolt-mcp.toml` from `--config`, else `PASSBOLT_MCP_CONFIG`, else the
configuration directory of the system (`~/.config/passbolt-mcp/` on Linux).

```toml
[passbolt]
server_url = "https://passbolt.example.com"
private_key_file = "/home/me/.config/passbolt-mcp/mcp-agent.asc"
passphrase_file = "/run/secrets/passbolt-mcp-passphrase"
# or: passphrase_command = ["pass", "show", "passbolt/mcp-agent"]
# if MFA is enforced: totp_secret_file = "/run/secrets/passbolt-mcp-totp"

[access]
write = false
secrets = false
share = false
allow_tools = []
deny_tools = []

[scope]
folders = ["Agents/CI"]

[audit]
file = "/var/log/passbolt-mcp/audit.jsonl"

[limits]
secrets_per_minute = 20
```

Without a file, every rule takes its default: read only, no secrets, no sharing, every folder the
MCP user can see.

## `[passbolt]`: the MCP user

| Key | What |
|---|---|
| `server_url` | the address of Passbolt |
| `private_key_file` | the armored private key of the MCP user |
| `passphrase_file` | a file holding its passphrase |
| `passphrase_command` | a command printing it, for a password manager or a keychain; instead of `passphrase_file` |
| `totp_secret_file` | the TOTP secret, when MFA is enforced for the MCP user |
| `server_key_file` | the server key pinned by `passbolt-mcp verify`; `server-key.asc` next to this file by default |
| `tls_skip_verify` | accept any certificate: for a test server only |
| `resolve` | `["host:port:address"]`, like `curl --resolve` |
| `timeout_seconds` | how long a request to the server may take; 60 by default. Listing every resource of a large server takes longer |

Relative paths are relative to the directory of this file. The first line of `passphrase_file`,
`totp_secret_file` or of the output of `passphrase_command` is used.

The passphrase is never a command-line argument nor a key of this file: both end up in shell
histories, process lists and backups.

## `[access]`: what the agent may do

| Key | Default | When `true` |
|---|---|---|
| `write` | `false` | create, update, move and delete resources and folders, tags, comments |
| `secrets` | `false` | read the secret of a resource (password, TOTP code…) |
| `share` | `false` | share resources and folders |
| `allow_tools` | `[]` | when not empty, only these tools |
| `deny_tools` | `[]` | never these tools, even if allowed above |

Reading the names, usernames, URIs and descriptions of what the MCP user can see is always allowed:
it is what the agent is for.

A tool the rules refuse is not offered to the agent at all: it does not appear in its list of
tools. The [tools](tools.md) page says which key each tool needs.

## `[scope]`: where the agent may act

| Key | Default | What |
|---|---|---|
| `folders` | `[]` | folders, by path or id, with their subfolders. When not empty, everything else is invisible to the agent, even if shared with the MCP user. |

A path starts at the highest folder the MCP user sees: `Agents/CI`. When two folders have the same
path, give the id. Resources at the root, in no folder, are outside any scope. A folder the MCP user
does not see stops `passbolt-mcp` at start, rather than leaving the agent with less than you meant.

## `[audit]`: what the agent did

| Key | Default | What |
|---|---|---|
| `file` | none | a JSON Lines file, created readable by its owner only: one line per call |

```json
{"time":"2026-06-29T15:20:04.600Z","tool":"get_secret","arguments":{"id":"8f0b…"},"outcome":"ok"}
{"time":"2026-06-29T15:20:05.112Z","tool":"get_secret","arguments":{"id":"8f0b…"},"outcome":"refused","code":"refused"}
```

`outcome` is `ok`, `refused` (by the rules), `error` (Passbolt or the arguments) or `protocol`
(an unknown tool). Only the arguments that name things are written: ids, folders, users, groups,
permissions, fields, `dry_run`. Never passwords, notes, descriptions, comments nor names.

Passbolt keeps its own record of what the MCP user did; this file adds what the agent asked and was
refused.

## `[limits]`: how fast

| Key | Default | What |
|---|---|---|
| `calls_per_minute` | `120` | every tool together |
| `secrets_per_minute` | `20` | `get_secret`, `get_totp_code` and `check_password_pwned` together |

Over a limit, a call is refused (`refused by the MCP rules: over limits.secrets_per_minute…`)
until the calls of the last minute make room: a runaway agent cannot read every secret at once.

## Effective rights

| Shared with the MCP user | Rules | The agent can |
|---|---|---|
| Read | default | list and read the metadata |
| Read | `secrets = true` | also read the secret |
| Read | `write = true` | still only read: Passbolt refuses the change |
| Update | `write = true` | change the resource |
| Owner | `share = true` | share it |
| anything, outside `scope.folders` | any | nothing: the resource is invisible |

`passbolt-mcp check` prints them for the MCP user and the rules of the configuration:

```
Logged in as MCP agent <mcp-agent@example.com>, role user
Sees 42 resources and 6 folders
Rules: read, secrets, no write, no share
Scope: 2 of the 6 folders
The agent can read 12 resources, read the secret of 12, change 0, share 0
```

It warns when the shares are broader than the rules: resources shared with the MCP user outside
`scope.folders` (hidden from the agent, but readable with its key), and resources the agent could
share because the MCP user owns them.

## Refusals

A tool the rules refuse is not offered to the agent. When a call reaches what the rules forbid, the
agent gets an error naming the rule, such as `refused by the MCP rules: outside scope.folders` or
`refused by the MCP rules: access.write is false`.

## Examples

A CI job reading a deployment password:

```toml
[access]
secrets = true
allow_tools = ["list_resources", "get_resource", "get_secret"]

[scope]
folders = ["Agents/CI"]
```

An assistant rotating passwords in one folder shared with **Update**:

```toml
[access]
write = true
secrets = true

[scope]
folders = ["Agents/Rotation"]

[audit]
file = "/var/log/passbolt-mcp/audit.jsonl"
```
