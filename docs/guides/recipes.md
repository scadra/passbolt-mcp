---
title: Rule recipes
sidebar_position: 4
---

# Rule recipes

Ready-made `[access]` and `[scope]` for common agents. Each one gives the least the task needs; the
[rules](../rules.md) page has every key. Share with the [MCP user](mcp-user.md) as little as the
recipe uses: the rules never give more than Passbolt does.

## An assistant that finds things

It answers "where is the login of the staging database?" without ever seeing a password.

```toml
# Nothing in [access]: read only, no secrets, the default.
[scope]
folders = ["Agents/Support"]
```

Share `Agents/Support` as **Read**.

## A deploy agent that reads a few passwords

```toml
[access]
secrets = true
deny_tools = ["check_password_pwned", "generate_password"]

[scope]
folders = ["Agents/CI"]

[audit]
file = "/var/log/passbolt-mcp/audit.jsonl"

[limits]
secrets_per_minute = 5
```

With a person at the keyboard, add `[confirm]` with `tools = ["get_secret"]`: they approve each
password the agent reads. A job that runs alone cannot be asked; keep it without.

Share `Agents/CI` as **Read**. Five secrets a minute is more than a deploy needs and less than an
agent emptying the folder would like; the audit file says which ones it read.

## TOTP codes only

The agent logs in to a service for you, with a code, never the seed nor the password:

```toml
[access]
secrets = true
allow_tools = ["whoami", "list_resources", "get_resource", "get_totp_code"]

[scope]
folders = ["Agents/2FA"]
```

`allow_tools` keeps the other secret tools (`get_secret` among them) away.

## A password hygiene audit

The `audit_password_hygiene` prompt lists expired, expiring and breached passwords. To find the
breached ones, it needs `check_password_pwned`, and only it:

```toml
[access]
secrets = true
allow_tools = ["whoami", "list_resources", "get_resource", "list_folders", "folder_tree",
               "check_password_pwned"]
```

Only the first five characters of the SHA-1 of each password leave the machine, to Have I Been
Pwned; the agent never sees a password.

## A rotation agent

It generates a new password, saves it with a dry run first, and checks it against breaches: the
`rotate_password` prompt.

```toml
[access]
secrets = true
write = true
deny_tools = ["delete_resource", "delete_folder", "delete_comment", "move_resource", "move_folder"]

[scope]
folders = ["Agents/Rotation"]

[confirm]
tools = ["update_resource"]
```

The person approves the new password before it is saved: the dry run first, then the real call.

Share `Agents/Rotation` as **Update**: changing a resource needs it, and nothing more is shared.

## An agent that organizes, without sharing

```toml
[access]
write = true

[scope]
folders = ["Agents/Inbox"]
```

It creates, renames and moves resources and folders inside `Agents/Inbox`, never their secrets.
Without `share`, a move keeps the permissions as they are: nothing becomes visible to more people.

## Check before connecting

```bash
passbolt-mcp check
```

prints what the agent will get with these rules and the shares of the MCP user, and warns when the
shares go further than the rules.
