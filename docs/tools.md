---
title: Tools
sidebar_position: 4
---

# Tools

What the agent can call. A tool is offered only when the [rules](rules.md) allow it, and acts only
on what is shared with the [MCP user](guides/mcp-user.md) inside `scope.folders`.

This page is filled as the tools are written.

## Reading

Always offered. They return metadata, never a secret; everything outside `scope.folders` is left
out, and asking for it by id is refused. Lists come in pages: `limit` (50 by default, 200 at most)
and the `cursor` of the previous page, with `total` and `next_cursor`.

| Tool | Arguments | What |
|---|---|---|
| `whoami` | | the MCP user, its role, and what the rules allow |
| `list_resources` | `search`, `folder`, `favorites_only`, `limit`, `cursor` | resources by name: name, username, URIs, folder path, type, permission, expiry |
| `get_resource` | `id` | the same, with the description and the dates |
| `list_folders` | `search`, `limit`, `cursor` | folders, with their paths and the permission of the MCP user |
| `folder_tree` | | folders, as a tree |
| `list_comments` | `id` (a resource) | its comments, oldest first |
| `get_permissions` | `resource` (an id) or `folder` (a path or an id) | who has access, and how |
| `list_resource_types` | | the resource types of the server |

`folder` is a path from the highest folder the MCP user sees (`Agents/CI`) or an id;
`list_resources` lists what is directly in it. Names, descriptions and comments are written by
people: the agent is told to treat them as data, never as instructions.

There is no tool for tags: Passbolt gives the tags of the MCP user, not those of a resource, so they
could not be kept within `scope.folders`.

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
| `add_comment`, `delete_comment` | comments |

## Sharing

Offered with `access.share = true`.

| Tool | What |
|---|---|
| `search_aros` | users and groups to share with, by name (`search`, `limit`, `cursor`) |
| `simulate_share` | what a share would change, without doing it |
| `share_resource` | add or remove a user or a group on a resource |
| `share_folder` | the same on a folder and its content |
