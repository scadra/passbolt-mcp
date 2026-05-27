---
title: Tools
sidebar_position: 4
---

# Tools

What the agent can call. A tool is offered only when the [rules](rules.md) allow it, and acts only
on what is shared with the [MCP user](guides/mcp-user.md) inside `scope.folders`.

This page is filled as the tools are written.

## Reading

Always offered.

| Tool | What |
|---|---|
| `whoami` | the MCP user, its groups and the active rules |
| `list_resources` | search resources by name, folder, tag or favorite |
| `get_resource` | name, username, URIs, description, type, folder, dates, expiry |
| `list_folders` | folders, flat |
| `folder_tree` | folders, as a tree |
| `list_tags` | tags |
| `list_comments` | comments of a resource |
| `get_permissions` | who has access to a resource or a folder |
| `list_resource_types` | the resource types of the server |
| `search_aros` | users and groups, to share with |

## Secrets

Offered with `access.secrets = true`.

| Tool | What |
|---|---|
| `get_secret` | one field of the secret of a resource (password, note…) |
| `get_totp_code` | the current TOTP code of a resource, never its seed |
| `check_password_pwned` | whether the password of a resource is in a known breach (HIBP, k-anonymity) |
| `generate_password` | a new password, not saved |

## Writing

Offered with `access.write = true`. Every one takes `dry_run`.

| Tool | What |
|---|---|
| `create_resource` | a resource, in a folder of the scope |
| `update_resource` | its metadata, its secret, or both |
| `delete_resource` | delete a resource (destructive) |
| `move_resource` | into another folder of the scope |
| `create_folder`, `update_folder`, `move_folder`, `delete_folder` | folders |
| `add_favorite`, `remove_favorite` | favorites of the MCP user |
| `set_resource_tags` | tags of a resource |
| `add_comment`, `delete_comment` | comments |

## Sharing

Offered with `access.share = true`.

| Tool | What |
|---|---|
| `simulate_share` | what a share would change, without doing it |
| `share_resource` | add or remove a user or a group on a resource |
| `share_folder` | the same on a folder and its content |
