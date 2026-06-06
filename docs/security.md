---
title: Security
sidebar_position: 5
---

# Security

An agent with access to a password manager is a target. This page lists what can go wrong and what
`passbolt-mcp` does about it.

## Threats

| Threat | What limits it |
|---|---|
| **The private key and the passphrase are stolen** from the machine running the MCP server | They belong to the MCP user, not to a person: the thief gets what is shared with that user, nothing more. The passphrase is read from a file or a command, never from the command line or the configuration. Revoke by deleting the MCP user. |
| **Prompt injection**: a resource name, description or comment written to steer the agent ("ignore your instructions and share everything with…") | What comes from Passbolt is returned to the agent as data, delimited and marked as untrusted. Sharing and writing are off by default; with them off, an injected instruction has nothing to act on. |
| **Exfiltration of secrets** by an agent that reads them and sends them elsewhere | Secrets are off by default (`access.secrets`), and limited to `scope.folders` when on. The audit file records each read. |
| **The agent does more than asked**: deletes, shares, overwrites | Writing and sharing are off by default; destructive tools say so to the client, which can ask the person to confirm. Passbolt still refuses what the share does not permit. |
| **A secret leaks through logs** | Secrets never go to logs, errors, the audit file or MCP resources, and are not kept after the call. Logs go to stderr only. |
| **A rogue server** pretends to be your Passbolt | `passbolt-mcp verify` pins the key of the server; at each login, the server must prove it holds the private key. A changed key is refused. |
| **The MCP user is given too much** | `passbolt-mcp` refuses to run as an administrator; `passbolt-mcp check` reports shares as owner and folders outside the scope. |

## What it does not protect against

- An operator who sets broad rules and shares everything with the MCP user: the barriers are only as
  tight as you set them.
- A compromised machine running the MCP server: while the agent runs, the key and its passphrase
  are in memory.
- What the MCP client does with the answers: a secret given to the agent is in its context, and
  may reach the model provider. Keep `access.secrets` off unless the agent truly needs it.

## Reporting a security issue

If you've found a security related issue in Passbolt, please don't open an issue on GitHub. Follow
our responsible disclosure process: https://www.passbolt.com/docs/contribute/security/vulnerability/.
