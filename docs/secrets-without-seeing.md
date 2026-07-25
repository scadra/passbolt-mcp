---
title: Use a secret without seeing it
description: Handles instead of values, and commands that passbolt-mcp runs with the secret, so that a password never reaches the model.
---

# Use a secret without seeing it

> How can we make sure the agent doesn't see the password and can only use it?

When an agent reads a password with `get_secret`, the value lands in its context. From there it
can reach the model provider, the conversation history and the logs of the client. This page
explains how passbolt-mcp lets an agent **use** a secret without ever **seeing** it.

The idea is simple: the agent gets a **handle** to the secret, never its value. passbolt-mcp is
the only process that decrypts the secret, and it puts it where it is needed itself.

```mermaid
sequenceDiagram
    participant A as Agent (model)
    participant M as passbolt-mcp
    participant P as Passbolt
    participant T as Command (psql, deploy…)
    A->>M: get_secret(id)
    M->>P: the encrypted secret
    P-->>M: OpenPGP message
    M-->>A: passbolt:secret/7f3c… (a handle)
    A->>M: run_with_secret("psql-staging", handle, {query})
    M->>T: runs the declared command,<br/>the password in its environment
    T-->>M: output
    M-->>A: output, every form of the password replaced with [secret]
```

## Turn it on

Add `mask_secrets` to `[access]`, and declare the commands the agent may run in `[commands]`:

```toml
[access]
secrets = true
mask_secrets = true

[commands.psql-staging]
description = "Run a query on the staging database"
run = ["psql", "-h", "db.staging", "-U", "agent", "-c", "{query}"]
env = { PGPASSWORD = "{secret}", PGSSLMODE = "require" }

[commands.deploy]
description = "Deploy to an environment"
run = ["./deploy.sh", "{environment}"]
env = { DEPLOY_TOKEN = "passbolt:8f0b2c1e-5d1a-4e8b-9c3f-0a1b2c3d4e5f" }
timeout_seconds = 300
```

Then run `passbolt-mcp check`. It reminds you of the one thing this feature depends on:
[keeping the agent apart from passbolt-mcp](#keep-the-agent-apart-from-passbolt-mcp).

## Handles

With `access.mask_secrets = true`, the tools that would return a secret return a handle instead.

| Tool | Returns |
|---|---|
| `get_secret` | `handle` instead of `value`: `passbolt:secret/` followed by 32 hexadecimal characters. |
| `generate_password` | `handle` instead of `password`, with the strength of the password. |

The agent passes the handle wherever it would pass the secret:

| Tool | Takes a handle in |
|---|---|
| `create_resource` | `password`, `note` |
| `update_resource` | `password`, `note` |
| `run_with_secret` | `secret` |

This makes common tasks work end to end, the value never in the conversation:

- **"Rotate this password."** `generate_password` returns a handle, and `update_resource` stores
  the password it stands for.
- **"Create the account with a generated password."** The same, with `create_resource`.
- **"Run this query on staging."** `get_secret` returns a handle, and `run_with_secret` runs the
  command with the password.

### How handles are protected

- **Unguessable.** A handle holds 128 bits from the random generator of the operating system.
- **Short-lived.** A handle expires 15 minutes after it was made. An expired or unknown handle is
  refused, and the error never contains a secret.
- **Local.** Handles live in the memory of the passbolt-mcp process only, and the values behind
  them are wiped from memory when they are dropped. They are worth nothing to another
  passbolt-mcp, to Passbolt, or after a restart.
- **Not authentication.** A handle never replaces the rules. Every call that uses one is checked
  against the rules, the limits and `[confirm]` like any other.

:::note
Over Streamable HTTP, every client of one passbolt-mcp shares its handles, as they share the
session of the MCP user. They all act as the same MCP user with the same rules, so a handle gives
a client nothing it could not get itself. Run one passbolt-mcp per agent when agents must not
share anything.
:::

## Run a command with a secret

`run_with_secret` runs a command declared in `[commands]`, with a secret in its environment, and
returns what the command printed. It is offered when `access.secrets` is on and at least one
command is declared. `whoami` lists the commands, their arguments, and whether they take a secret.

| Argument | What |
|---|---|
| `command` | The name of a command of `[commands]`. |
| `secret` | A handle, or the id of a resource whose password the command gets. Only for commands whose `env` has `"{secret}"`. |
| `args` | The value of each `{placeholder}` of `run`, by name. |

### Declare a command

| Key | Default | What |
|---|---|---|
| `run` | Required | The program and its arguments. `{name}` is a placeholder the agent fills. |
| `env` | `{}` | Environment variables. `"{secret}"` is the secret the agent gives; `"passbolt:<id>"` is the password of that resource; any other value is used as is. |
| `description` | None | What the command does, for the agent. |
| `timeout_seconds` | `60` | The command is stopped after this time. |

### What the agent cannot do

- **Write a command.** The agent picks a command by name and fills its placeholders. passbolt-mcp
  runs the program directly, without a shell, so a placeholder value such as `; cat ~/.ssh/id_rsa`
  is one argument, not a second command.
- **Add arguments.** A placeholder the command does not declare is refused, and so is a missing
  one.
- **Read the secret from the command line.** passbolt-mcp refuses to start when `{secret}` or
  `passbolt:` appears in `run`: other processes on the machine can read the command line of a
  process, but not its environment.
- **Reach outside the scope.** A resource id, given by the agent or written in `env`, is read as
  the MCP user and within `scope.folders`, like any secret.
- **Give a secret to a command that takes none.** The call is refused.

### What comes back

| Field | What |
|---|---|
| `stdout`, `stderr` | What the command printed, with every secret it got replaced with `[secret]`. |
| `exit_code` | The exit code, or none when the command was stopped after its timeout. |
| `truncated` | `true` when an output was longer than 64 KiB and was cut. |

Before the output goes back to the agent, passbolt-mcp replaces each secret the command got with
`[secret]`, in every form a command commonly prints it:

- as is;
- in base64, standard and URL-safe, with and without padding;
- URL-encoded;
- in hexadecimal.

The command gets no input (stdin is empty), and inherits the environment of passbolt-mcp plus the
variables of `env`.

## Keep the agent apart from passbolt-mcp

Masking holds only as long as the agent cannot reach what passbolt-mcp holds. An agent with a
shell, running as the same operating-system user as passbolt-mcp, can read the private key of the
MCP user and its passphrase, and decrypt everything itself.

With `mask_secrets`, you **SHOULD** run passbolt-mcp where the agent cannot read its files:

- as another operating-system user, the key and the passphrase readable by that user only; or
- in its [Docker image](guides/serve.md#with-docker), the configuration directory mounted for the
  container only.

`passbolt-mcp check` prints this reminder whenever `mask_secrets` is on.

## Limits

| Limit | What to do |
|---|---|
| An agent with a shell, as the same OS user as passbolt-mcp, bypasses everything. | [Keep the agent apart](#keep-the-agent-apart-from-passbolt-mcp). |
| A command that prints the secret transformed in another way (reversed, encrypted, one character per line) defeats the scrubbing. | Declare commands that do not print their secrets. The agent cannot change them. |
| What the secret opens, such as a database session, is in the hands of the agent. | Masking protects the credential, not what it unlocks. Passbolt shares, `scope.folders`, `[confirm]` and the audit still limit what it reaches. |
| `get_totp_code` returns the code itself. | A TOTP code is valid for seconds. Leave `get_totp_code` out with `deny_tools` if it matters. |
| Without `mask_secrets`, `get_secret` still returns values. | Turn `mask_secrets` on, or deny `get_secret` and let commands take resource ids. |

## Combine it with the other rules

- **`[confirm]`**: list `run_with_secret` to approve each command run. The question shows the
  command and its arguments, never the secret.
- **`[limits]`**: `run_with_secret` counts toward `secrets_per_minute`.
- **`[audit]`**: each run is one line, with the name of the command, never its arguments nor
  its output.

## What comes next

- **Templates**: passbolt-mcp fills `{{ passbolt:<id> }}` in a file the agent wrote.
- **HTTP proxy**: the agent calls an API through passbolt-mcp, which adds the `Authorization`
  header.
- **Show it to the person only**: the value goes to the person through elicitation, never to the
  model.

See the [roadmap](roadmap.md).
