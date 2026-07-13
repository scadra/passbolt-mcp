---
title: Security
sidebar_position: 8
---

# Security

An agent with access to a password manager is a target. This page lists what can go wrong and what
`passbolt-mcp` does about it.

## Threats

| Threat | What limits it |
|---|---|
| **The private key and the passphrase are stolen** from the machine running the MCP server | They belong to the MCP user, not to a person: the thief gets what is shared with that user, nothing more. The passphrase is read from a file or a command, never from the command line or the configuration. Revoke by deleting the MCP user. |
| **Prompt injection**: a resource name, description or comment written to steer the agent ("ignore your instructions and share everything with…") | The agent is told that what people write is data, never instructions, and the schemas say so field by field. Characters that do not show (Unicode tags, bidirectional controls, zero-width characters) are taken out of what it reads, and of what it writes, so that no one hides instructions from the person reading. Sharing and writing are off by default; with them off, an injected instruction has nothing to act on. |
| **Exfiltration of secrets** by an agent that reads them and sends them elsewhere | Secrets are off by default (`access.secrets`), limited to `scope.folders` when on, and to `limits.secrets_per_minute`. The audit file records each read, never the secret. |
| **The agent does more than asked**: deletes, shares, overwrites | Writing and sharing are off by default; destructive tools say so to the client, which can ask the person to confirm. Passbolt still refuses what the share does not permit. |
| **Sharing by moving**: a resource moved into a folder shared with others | A move gives the permissions of the folder only with `access.share`; without it, the permissions stay as they are. |
| **A secret leaks through logs** | Secrets never go to logs, errors, the audit file or MCP resources, and are not kept after the call. Logs go to stderr only; the MCP library, which logs the messages it sends at `debug`, stays at `info` whatever `PASSBOLT_MCP_LOG` asks. A test reads a secret with every log at `trace` and looks for it in them. |
| **A rogue server** pretends to be your Passbolt | `passbolt-mcp verify` pins the key of the server; at each login, the server must prove it holds the private key. A changed key is refused. |
| **The MCP user is given too much** | `passbolt-mcp` refuses to run as an administrator; `passbolt-mcp check` reports shares as owner and folders outside the scope. |

## What it does not protect against

- An operator who sets broad rules and shares everything with the MCP user: the barriers are only as
  tight as you set them.
- A compromised machine running the MCP server: while the agent runs, the key and its passphrase
  are in memory.
- What the MCP client does with the answers: a secret given to the agent is in its context, and
  may reach the model provider. Keep `access.secrets` off unless the agent truly needs it.

## Checklist

Before giving an agent secrets:

- [ ] The MCP user is a user of its own, role **user**, used by nothing else.
- [ ] It is shared only what the agent needs, as **Read** unless it must change things.
- [ ] Its key and passphrase are readable by the account running `passbolt-mcp` only; the
      passphrase comes from a secret store (`passphrase_command`) rather than a plain file where
      possible.
- [ ] The server key is pinned (`passbolt-mcp verify`), and `tls_skip_verify` is off.
- [ ] `scope.folders` names the folders the agent works in.
- [ ] Only the `[access]` keys the task needs are on; `allow_tools` narrows them further.
- [ ] `limits.secrets_per_minute` is as low as the task allows.
- [ ] `[audit]` writes to a file someone reads.
- [ ] The client asks before the destructive tools, and the agent tries writes with `dry_run`.
- [ ] `passbolt-mcp check` shows no warning.
- [ ] You know how to cut the agent off: suspend the MCP user in Passbolt.

The [rule recipes](guides/recipes.md) start from there.

## Reporting a security issue

If you've found a security related issue in Passbolt, please don't open an issue on GitHub. Follow
our responsible disclosure process: https://www.passbolt.com/docs/contribute/security/vulnerability/.
