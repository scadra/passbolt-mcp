---
title: Quick start
description: From nothing to an agent reading one folder of Passbolt, in five steps.
---

# Quick start

In this tutorial, you connect Claude Code to Passbolt through passbolt-mcp. At the end, you can ask
the agent which resources it sees in one folder. The agent reads names, usernames and URIs only,
never a password.

## Prerequisites

- passbolt-mcp, [installed](installation.md).
- A Passbolt server, and an administrator account on it for the first step.
- [Claude Code](https://code.claude.com), or another [MCP client](../guides/serve.md).

## 1. Create the MCP user

The agent never uses your account. It works through a Passbolt user created for it, the
*MCP user*.

1. In Passbolt, an administrator invites a user for the agent, such as `mcp-agent@example.com`,
   with the role **User**.
2. Open the invitation in a browser profile of its own, choose a passphrase, and download the
   recovery kit. The kit is the private key of the MCP user, such as `mcp-agent.asc`.
3. Share a folder with the MCP user, such as `Agents/CI`, as **Read**.

[The MCP user](../guides/mcp-user.md) explains each step in detail.

## 2. Write the configuration

Create `~/.config/passbolt-mcp/passbolt-mcp.toml`. Put the key and a file holding the passphrase
next to it, readable by you only:

```toml
[passbolt]
server_url = "https://passbolt.example.com"
private_key_file = "mcp-agent.asc"
passphrase_file = "mcp-agent.passphrase"

[scope]
folders = ["Agents/CI"]
```

Without an `[access]` section, the agent can only read metadata. [Rules](../rules.md) describes
every key.

## 3. Pin the server key

```bash
passbolt-mcp verify
```

The server proves it holds its OpenPGP key, and passbolt-mcp saves the key next to the
configuration, in `server-key.asc`. Each later login checks it again.

## 4. Check what the agent gets

```bash
passbolt-mcp check
```

passbolt-mcp logs in as the MCP user and prints who it is, what the rules allow and what it sees,
with a warning for each risky rule.

## 5. Connect the agent

```bash
claude mcp add passbolt -- passbolt-mcp
```

Ask Claude: "Which resources can you see in Passbolt?"

## Next steps

- **Let the agent use passwords** without seeing them: [Use a secret without seeing it](../secrets-without-seeing.md).
- **Learn what the agent can call**: [Tools](../tools.md) and [Example prompts](../examples.md).
- **Before giving it secrets**, read the [Security best practices](../security.md).
