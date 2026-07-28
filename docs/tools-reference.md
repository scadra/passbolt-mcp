---
title: Tools reference
---

<!-- Written by `UPDATE_DOCS=1 cargo test tools_reference`: do not edit. -->

# Tools reference

Every tool and its arguments, from the schemas the agent gets. [Tools](tools.md) says how they work together.

## Reading

Always offered.

### `folder_tree`

The folders the agent can see, as a tree.

*Read only.*

### `get_permissions`

Who has access to a resource or a folder, and how.

*Read only.*

| Argument | Type | Description |
|---|---|---|
| `folder` | string | A folder, by path (`Agents/CI`) or id. |
| `resource` | string | The id of a resource. |

### `get_resource`

A resource: name, username, URIs, description, folder, dates. Never its secret.

*Read only.*

| Argument | Type | Description |
|---|---|---|
| `id` (required) | string | The id of the resource. |

### `list_comments`

The comments on a resource, oldest first.

*Read only.*

| Argument | Type | Description |
|---|---|---|
| `id` (required) | string | The id of the resource. |

### `list_folders`

The folders the agent can see, with their paths.

*Read only.*

| Argument | Type | Description |
|---|---|---|
| `cursor` | string | The `next_cursor` of the previous page, for the next one. |
| `limit` | number | At most this many items: 50 by default, 200 at most. |
| `search` | string | Part of the name, any case. |

### `list_resource_types`

The resource types of the server: what a resource can hold.

*Read only.*

### `list_resources`

The resources the agent can see, with their names: never their secrets.

*Read only.*

| Argument | Type | Description |
|---|---|---|
| `cursor` | string | The `next_cursor` of the previous page, for the next one. |
| `favorites_only` | boolean | Only the favorites of the MCP user. |
| `folder` | string | Only the resources directly in this folder, by path (`Agents/CI`) or id. |
| `limit` | number | At most this many items: 50 by default, 200 at most. |
| `search` | string | Part of the name, any case. |

### `whoami`

The Passbolt user this server acts as, and what its rules allow.

*Read only.*

## Secrets

With `access.secrets = true`.

### `call_api`

Call an API of the configuration, passbolt-mcp adding the secret to the request: the agent never sees it, and the answer comes back with the secret taken out. `whoami` lists the APIs.

*Destructive.*

| Argument | Type | Description |
|---|---|---|
| `api` (required) | string | The name of an API of the configuration, as `whoami` lists it. |
| `body` |  | The body of the request, sent as JSON. |
| `method` | string | `GET` by default; one of the methods the API allows. |
| `path` (required) | string | The path under the base URL of the API, such as `repos/acme/app/issues`. |
| `query` | object | Query parameters, by name. |
| `secret` | string | A handle from `get_secret` or `generate_password`, or the id of a resource, whose password goes in the request; for the APIs that take one. |

### `check_password_pwned`

Whether the password of a resource is in a known breach, by k-anonymity: only the first five characters of its SHA-1 leave, to Have I Been Pwned.

*Read only.*

| Argument | Type | Description |
|---|---|---|
| `id` (required) | string | The id of the resource. |

### `fill_template`

Fill a template of the configuration: passbolt-mcp reads the file the agent wrote, puts each secret where `{{ passbolt:<resource id> }}`, `{{ passbolt:<resource id>#<field> }}` or a handle stands, and writes the target, which the agent cannot read. `whoami` lists the templates.

*Destructive.*

| Argument | Type | Description |
|---|---|---|
| `template` (required) | string | The name of a template of the configuration, as `whoami` lists it. |

### `generate_password`

A new password, passphrase or PIN code, following the password policy of the organization unless asked otherwise, with its strength; saved nowhere.

*Read only.*

| Argument | Type | Description |
|---|---|---|
| `kind` | Kind | `password`, `passphrase` or `pin`; the generator of the password policy of the organization by default. |
| `length` | number | Characters of a password, digits of a PIN code (4 to 12, 6 by default); the policy's by default. |
| `symbols` | boolean | `false` for letters and digits only, for a service that refuses symbols; the policy's character sets by default. |
| `words` | number | Words of a passphrase; the policy's by default. |

### `get_secret`

One field of the secret of a resource: its password by default.

*Read only.*

| Argument | Type | Description |
|---|---|---|
| `field` | string | The field of the secret: `password` by default, or `description`, the note of a v5 resource. |
| `id` (required) | string | The id of the resource. |

### `get_totp_code`

The current TOTP code of a resource, or a handle to it; never its seed.

*Read only.*

| Argument | Type | Description |
|---|---|---|
| `id` (required) | string | The id of the resource. |

### `run_with_secret`

Run a command of the configuration with a password in its environment, which the agent never sees: what the command prints comes back with the password taken out. `whoami` lists the commands.

*Destructive.*

| Argument | Type | Description |
|---|---|---|
| `args` | object | The value of each placeholder of the command, by name. |
| `command` (required) | string | The name of a command of the configuration, as `whoami` lists it. |
| `secret` | string | A handle from `get_secret` or `generate_password`, or the id of a resource, whose password the command gets; for the commands that take one. |

### `show_secret`

Show one field of the secret of a resource to the person, in the client: the agent never sees it. For when the person needs the password itself.

*Read only.*

| Argument | Type | Description |
|---|---|---|
| `field` | string | The field of the secret: `password` by default, or `description`, the note of a v5 resource. |
| `id` (required) | string | The id of the resource. |

## Writing

With `access.write = true`.

### `add_comment`

Comment on a resource, as the MCP user.

| Argument | Type | Description |
|---|---|---|
| `content` (required) | string | The text of the comment. |
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `id` (required) | string | The id of the resource. |
| `parent_id` | string | The comment it answers. |

### `create_folder`

Create a folder; in a parent folder, it gets the permissions of the parent.

| Argument | Type | Description |
|---|---|---|
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `name` (required) | string | The name of the folder. |
| `parent` | string | The folder to create it in, by path or id; the root when absent. |

### `create_resource`

Create a resource, its secret encrypted for the MCP user; in a folder, it gets the permissions of the folder.

| Argument | Type | Description |
|---|---|---|
| `description` | string | The description, in the metadata: not secret. |
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `expiry` | string | Expiry of the password, RFC 3339. |
| `folder` | string | The folder to create it in, by path or id; the root when absent. |
| `name` (required) | string | The name, as Passbolt shows it. |
| `note` | string | The secret note of a v5 resource. |
| `password` | string | From `generate_password`, for instance; a handle works too. |
| `resource_type` | string | Its type, by slug (`list_resource_types`); by default, from the fields given. |
| `uris` | array of string | The URIs; the first is the main one. |
| `username` | string | The username, or login. |

### `delete_comment`

Delete a comment the MCP user wrote on a resource.

*Destructive.*

| Argument | Type | Description |
|---|---|---|
| `comment_id` (required) | string | The id of the comment, from `list_comments`. |
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `id` (required) | string | The id of the resource. |

### `delete_folder`

Delete a folder, for everyone it is shared with; with `with_content`, its subfolders and resources too. Try it with `dry_run` first.

*Destructive.*

| Argument | Type | Description |
|---|---|---|
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `folder` (required) | string | The folder, by path or id. |
| `with_content` | boolean | Also delete its subfolders and the resources the MCP user may delete in them; otherwise they move to the root, as do those it may not delete. |

### `delete_resource`

Delete a resource, for everyone it is shared with. Try it with `dry_run` first.

*Destructive.*

| Argument | Type | Description |
|---|---|---|
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `id` (required) | string | The id of the resource. |

### `move_folder`

Move a folder, with its content, into another folder of the scope.

| Argument | Type | Description |
|---|---|---|
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `folder` (required) | string | The folder, by path or id. |
| `parent` | string | The folder to move it into, by path or id; the root when absent. |

### `move_resource`

Move a resource to another folder of the scope.

| Argument | Type | Description |
|---|---|---|
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `folder` | string | The folder to move it to, by path or id; the root when absent. |
| `id` (required) | string | The id of the resource. |

### `rename_folder`

Rename a folder.

| Argument | Type | Description |
|---|---|---|
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `folder` (required) | string | The folder, by path or id. |
| `name` (required) | string | The new name. |

### `set_favorite`

Mark a resource as a favorite of the MCP user, or unmark it.

| Argument | Type | Description |
|---|---|---|
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `favorite` (required) | boolean | `true` to mark it, `false` to unmark it. |
| `id` (required) | string | The id of the resource. |

### `update_resource`

Change the fields given of a resource; a new secret is encrypted for everyone with access.

| Argument | Type | Description |
|---|---|---|
| `clear_expiry` | boolean | Remove the expiry. |
| `description` | string | The new description. |
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `expiry` | string | Expiry of the password, RFC 3339. |
| `id` (required) | string | The id of the resource. |
| `name` | string | The new name. |
| `note` | string | The new secret note of a v5 resource. |
| `password` | string | The new password, or a handle to it: encrypted again for everyone with access. |
| `uris` | array of string | The new URIs, all of them. |
| `username` | string | The new username. |

## Sharing

With `access.share = true`.

### `search_aros`

Users and groups to share with.

*Read only.*

| Argument | Type | Description |
|---|---|---|
| `cursor` | string | The `next_cursor` of the previous page, for the next one. |
| `limit` | number | At most this many items: 50 by default, 200 at most. |
| `search` (required) | string | Part of a name, a username or a group name. |

### `share_folder`

Share a folder with users and groups, or revoke their access; the MCP user must own it. With `with_content` (the default), also on what it owns inside.

*Destructive.*

| Argument | Type | Description |
|---|---|---|
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `folder` (required) | string | The folder, by path or id. |
| `groups` | array of string | Ids of groups, from `search_aros`. |
| `permission` (required) | string | `read`, `update`, `owner`, or `none` to revoke their access. |
| `users` | array of string | Ids of users, from `search_aros`. |
| `with_content` | boolean | Also on the subfolders and resources the MCP user owns, as the browser extension does. |

### `share_resource`

Share a resource with users and groups, or revoke their access; the MCP user must own it. Those who gain access get the secret, encrypted for them.

*Destructive.*

| Argument | Type | Description |
|---|---|---|
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `groups` | array of string | Ids of groups, from `search_aros`. |
| `id` (required) | string | The id of the resource. |
| `permission` (required) | string | `read`, `update`, `owner`, or `none` to revoke their access. |
| `users` | array of string | Ids of users, from `search_aros`. |

### `simulate_share`

Who would get access to a resource if it were shared so, group members included; shares nothing.

*Read only.*

| Argument | Type | Description |
|---|---|---|
| `dry_run` | boolean | Say what it would do, and do nothing. |
| `groups` | array of string | Ids of groups, from `search_aros`. |
| `id` (required) | string | The id of the resource. |
| `permission` (required) | string | `read`, `update`, `owner`, or `none` to revoke their access. |
| `users` | array of string | Ids of users, from `search_aros`. |
