---
title: Tools
sidebar_position: 5
---

# Tools

What the agent can call: tools, and the [resources](#resources) and [prompts](#prompts) below. A tool is offered only when the [rules](rules.md) allow it, and acts only
on what is shared with the [MCP user](guides/mcp-user.md) inside `scope.folders`.

Every argument of every tool, from the schemas the agent gets: the
[tools reference](tools-reference.md).

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

Offered with `access.secrets = true`, within `scope.folders`. A secret goes to the agent and
nowhere else: not to the logs, at any level, nor to the errors.

| Tool | Arguments | What |
|---|---|---|
| `get_secret` | `id`, `field` | one field of the secret: `password` by default, `description` (the note of a v5 resource), `pin_code`… An unknown field gets the names of those the secret has, never their values. |
| `get_totp_code` | `id` | the current TOTP code and how long it stays valid; never the seed, which `get_secret` refuses too |
| `check_password_pwned` | `id` | whether the password is in a known breach, and how many times: only the first five characters of its SHA-1 go to [Have I Been Pwned](https://haveibeenpwned.com/Passwords) |
| `generate_password` | `length` (20; 8 to 128), `lowercase`, `uppercase`, `digits`, `symbols` | a random password, with at least one character of each class asked; saved nowhere |

## Writing

Offered with `access.write = true`, within `scope.folders`. Every write takes `dry_run`: it then
says what it would do (`"would delete the resource Staging DB…"`) and does nothing. A write the
permission of the MCP user does not allow is refused before reaching Passbolt (`forbidden`):
changing, moving or deleting needs **update**, commenting **read**. Messages name the fields that
change, never their values.

| Tool | Arguments | What |
|---|---|---|
| `create_resource` | `name`, `username`, `uris`, `description`, `password`, `note`, `folder`, `resource_type`, `expiry` | a resource, its secret encrypted for the MCP user; in a folder, it gets the permissions of the folder |
| `update_resource` | `id` and the fields to change, `clear_expiry` | the fields given; a new secret is encrypted for everyone with access |
| `delete_resource` | `id` | deletes it for everyone (destructive) |
| `move_resource` | `id`, `folder` (the root when absent) | into another folder of the scope; its permissions: see [moves](#moves) |
| `create_folder` | `name`, `parent` | a folder; in a parent, it gets the permissions of the parent |
| `rename_folder` | `folder`, `name` | |
| `move_folder` | `folder`, `parent` | with its content; see [moves](#moves) |
| `delete_folder` | `folder`, `with_content` | without `with_content`, its content moves up to its parent (destructive) |
| `set_favorite` | `id`, `favorite` | mark or unmark a favorite of the MCP user |
| `add_comment` | `id`, `content`, `parent_id` | a comment, by the MCP user |
| `delete_comment` | `id`, `comment_id` | a comment of the MCP user (destructive) |

The destructive tools say so to the client, which can ask the person before calling them.

## Sharing

Offered with `access.share = true`, within `scope.folders`. Only an owner shares, as in Passbolt:
a share on something the MCP user does not own is refused (`forbidden`). `permission` is `read`,
`update`, `owner`, or `none` to revoke.

| Tool | Arguments | What |
|---|---|---|
| `search_aros` | `search`, `limit`, `cursor` | users and groups to share with, by name: their ids |
| `simulate_share` | as `share_resource` | the users who would get access, group members included; shares nothing |
| `share_resource` | `id`, `users`, `groups` (ids), `permission`, `dry_run` | those who get access get the secret, encrypted for them |
| `share_folder` | `folder`, `users`, `groups`, `permission`, `with_content` (on by default), `dry_run` | with `with_content`, also on the subfolders and resources the MCP user owns in it |

### Moves

With `access.share` and when the MCP user owns it, `move_resource` and `move_folder` give what moves
the permissions of its new folder, as the browser extension does. Otherwise a move leaves the
permissions as they are, and says so: moving a resource into a shared folder never shares it
without `access.share`.

## Resources

MCP resources, for the clients that attach context rather than call tools. Metadata only, never a
secret, within `scope.folders`, counted in `limits.calls_per_minute` and written to the audit file
(`resources/read`).

| URI | What |
|---|---|
| `passbolt://resource/{id}` | what `get_resource` gives |
| `passbolt://folder/{id}` | the folder and the resources directly in it (200 at most; `truncated` says when there are more) |

`resources/list` lists the folders the agent can see, in pages; resources are read from their id,
as `list_resources` gives it: listing every resource would decrypt them all at each listing.

## Prompts

Tasks the agent runs with the tools above, written for the rules of the server.

| Prompt | Arguments | Offered | What |
|---|---|---|---|
| `audit_password_hygiene` | `folder` | always | expired, expiring and, with `access.secrets`, breached passwords, as a table; it paces the breach checks to `limits.secrets_per_minute` |
| `rotate_password` | `id` | with `access.write` and `access.secrets` | a new password, generated, saved with a dry run first, checked against breaches; the person then sets it on the service |
