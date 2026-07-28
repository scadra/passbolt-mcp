---
title: Rule recipes
---

# Rule recipes

This page gives ready-made rules for common agents. Each recipe gives the least the task needs.
[Rules](../rules.md) describes every key.

Share with the [MCP user](mcp-user.md) only what the recipe uses: the rules never give more than
Passbolt does.

## An assistant that finds things

It answers "where is the login of the staging database?" without ever seeing a password.

```toml
# Nothing in [access]: read only, no secrets, the default.
[scope]
folders = ["Agents/Support"]
```

Share `Agents/Support` as **Read**.

## A deploy agent that uses passwords without seeing them

The agent deploys and queries the staging database. It never sees a password: passbolt-mcp runs
the commands with the password in their environment. See
[Use a secret without seeing it](../secrets-without-seeing.md).

```toml
[access]
secrets = true
allow_tools = ["whoami", "list_resources", "get_resource", "run_with_secret"]

[commands.deploy-staging]
description = "Deploy a version to staging"
run = ["./deploy.sh", "staging", "{version}"]
env = { DEPLOY_TOKEN = "passbolt:8f0b2c1e-5d1a-4e8b-9c3f-0a1b2c3d4e5f" }
timeout_seconds = 600

[commands.psql-staging]
description = "Run a read-only query on the staging database"
run = ["psql", "-h", "db.staging", "-U", "readonly", "-c", "{query}"]
env = { PGPASSWORD = "{secret}" }

[scope]
folders = ["Agents/CI"]

[audit]
file = "/var/log/passbolt-mcp/audit.jsonl"

[limits]
secrets_per_minute = 5
```

Run passbolt-mcp as another OS user than the agent, or in Docker, so that the agent cannot read the
key of the MCP user.

## A deploy agent that reads a few passwords

When the agent must have the password itself, for a tool passbolt-mcp cannot run. This is the one
case for `reveal_secrets`:

```toml
[access]
secrets = true
reveal_secrets = true
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

The agent logs in to a service for you, with a code, never the seed nor the password. It types the
code itself, so it sees it:

```toml
[access]
secrets = true
reveal_secrets = true
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

It generates a new password that follows the password policy of the organization, saves it with a
dry run first, and checks it against breaches: the `rotate_password` prompt. The new password goes from `generate_password` to `update_resource` as a handle, and the agent
never sees it.

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
