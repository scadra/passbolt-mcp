---
title: Quick start
sidebar_position: 2
---

# Quick start

From nothing to an agent reading one folder of Passbolt, in five steps. You need a Passbolt
administrator for the first one.

## 1. Create the MCP user

In Passbolt, an administrator invites a user for the agent, `mcp-agent@example.com`, with the role
**user**. Open the invitation in a browser profile of its own, choose a passphrase, and download
the recovery kit: the private key of the MCP user (`mcp-agent.asc`). Then share a folder with
it, `Agents/CI`, as **Read**. Step by step: [the MCP user](../guides/mcp-user.md).

## 2. Write the configuration

`~/.config/passbolt-mcp/passbolt-mcp.toml`, with the key and the passphrase next to it, readable by
you only:

```toml
[passbolt]
server_url = "https://passbolt.example.com"
private_key_file = "mcp-agent.asc"
passphrase_file = "mcp-agent.passphrase"

[scope]
folders = ["Agents/CI"]
```

Read only: the agent sees names, usernames and URIs, never a password. Every rule:
[rules](../rules.md).

## 3. Pin the server key

```bash
passbolt-mcp verify
```

The server proves it holds its OpenPGP key, and the key is saved next to the configuration
(`server-key.asc`): each later login checks it again.

## 4. Check what the agent gets

```bash
passbolt-mcp check
```

It logs in as the MCP user and prints who it is, what the rules allow, and what it sees, with a
warning on each risky rule.

## 5. Connect the agent

```bash
claude mcp add passbolt -- passbolt-mcp
```

Ask Claude "which resources can you see in Passbolt?". Other clients: [clients](../guides/serve.md).

## Then

- Let it read passwords: `access.secrets = true`, see [rules](../rules.md).
- Know what it can call: [tools](../tools.md).
- Before giving it secrets: [security](../security.md).
