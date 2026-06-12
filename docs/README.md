---
title: Introduction
sidebar_position: 1
slug: /
---

# passbolt-mcp

An [MCP](https://modelcontextprotocol.io) server for [Passbolt](https://www.passbolt.com), the open
source password manager for teams. It lets an AI agent find, read, create and share secrets in
Passbolt, through a Passbolt user dedicated to the agent and within rules you set.

It is written in Rust, on [`passbolt-sdk`](https://crates.io/crates/switch-case-passbolt-sdk) for
the Passbolt API and OpenPGP, and [`rmcp`](https://github.com/modelcontextprotocol/rust-sdk) for the
protocol. It logs in as the browser extension does (GPGAuth), encrypts and decrypts on the client,
and understands both v4 resources and v5 encrypted metadata.

## How it works

```
  agent ──MCP──▶ passbolt-mcp ──────────────▶ Passbolt
                 the MCP rules                the MCP user
                 what it may DO               what it may SEE
```

- **The MCP user** is an ordinary Passbolt user, with role `user`, created for the agent. The agent
  sees only what is shared with it. See [the MCP user](guides/mcp-user.md).
- **The MCP rules** are the configuration of `passbolt-mcp`: which tools the agent gets, whether it
  may write, read secrets or share, and in which folders. See [rules](rules.md).

The agent gets what both allow, never more.

## Where to start

- **Set up an agent**: [create the MCP user](guides/mcp-user.md) in Passbolt, share a folder with
  it, then [write the rules](rules.md) and [connect the agent](guides/serve.md).
- **Know what the agent can call**: the [tools](tools.md).
- **Before giving it secrets**: read [security](security.md).
- **Work on passbolt-mcp itself**: [development](contributing/development.md).

## License

AGPL-3.0-or-later. The name "Passbolt" is a registered trademark of Passbolt SA.
