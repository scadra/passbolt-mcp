---
title: Rules
sidebar_position: 3
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

## `[audit]`: what the agent did

| Key | Default | What |
|---|---|---|
| `file` | none | a JSON Lines file: one line per call, with the tool, the ids, the decision of the rules and the result; never a secret |

Passbolt keeps its own record of what the MCP user did; this file adds what the agent asked and was
refused.

## Effective rights

| Shared with the MCP user | Rules | The agent can |
|---|---|---|
| Read | default | list and read the metadata |
| Read | `secrets = true` | also read the secret |
| Read | `write = true` | still only read: Passbolt refuses the change |
| Update | `write = true` | change the resource |
| Owner | `share = true` | share it |
| anything, outside `scope.folders` | any | nothing: the resource is invisible |

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
