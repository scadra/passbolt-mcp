---
title: Example prompts
sidebar_position: 7
---

# Example prompts

What to ask the agent, and the [tools](tools.md) it calls. Each prompt works only when the
[rules](rules.md) offer those tools and the resources are shared with the MCP user.

## Finding things

Read only, the default.

| Ask | It calls |
|---|---|
| "Which Passbolt user are you, and what may you do?" | `whoami` |
| "Where is the login of the staging database?" | `list_resources` |
| "Show me the folders you can see, as a tree." | `folder_tree` |
| "Who has access to `Staging DB`, and with which permission?" | `list_resources`, `get_permissions` |
| "What do the comments on `Grafana admin` say?" | `list_comments` |
| "Which passwords in `Agents/CI` expire this month?" | `list_resources` |

## Secrets

With `access.secrets = true`.

| Ask | It calls |
|---|---|
| "Connect to the staging database with its password and list the tables." | `list_resources`, `get_secret` |
| "Give me the current 2FA code of the AWS root account." | `get_totp_code` |
| "Is any password in `Agents/CI` in a known breach?" | `list_resources`, `check_password_pwned` |
| "Generate a 32-character password without symbols." | `generate_password` |

## Writing

With `access.write = true`. Ask for a dry run first: the agent then says what it would do.

| Ask | It calls |
|---|---|
| "Save a new resource `Sentry token` in `Agents/CI`, with this token." | `create_resource` |
| "Rotate the password of `Staging DB`: generate one, show me the dry run, then save it." | `generate_password`, `update_resource` |
| "Move everything about the old cluster into `Archive`." | `list_resources`, `move_resource` |
| "Delete `Agents/CI/tmp` and what is in it, dry run first." | `delete_folder` |

## Sharing

With `access.share = true`, on what the MCP user owns.

| Ask | It calls |
|---|---|
| "Who would see `Staging DB` if I shared it with the Ops group?" | `search_aros`, `simulate_share` |
| "Share `Agents/CI` with Betty, read only." | `search_aros`, `share_folder` |
| "Remove Carol's access to `Grafana admin`." | `search_aros`, `share_resource` (`none`) |

## Prompts

Clients that show MCP prompts (a `/` menu in Claude Code and Claude Desktop) offer the ones of
`passbolt-mcp`:

- `audit_password_hygiene`: expired, expiring and breached passwords of a folder, as a table.
- `rotate_password`: a new password for a resource, with a dry run, checked against breaches. With
  `access.write` and `access.secrets` only.
