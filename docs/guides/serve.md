---
title: Connect an agent
sidebar_position: 3
---

# Connect an agent

Once the [MCP user](mcp-user.md) exists, its key and passphrase are in place, the
[rules](../rules.md) are written and the server is verified:

```bash
passbolt-mcp verify   # once
passbolt-mcp check    # who the agent will be, and what it can do
```

## Over stdio

The MCP client starts `passbolt-mcp` and talks to it on stdin and stdout. It logs in as the MCP user
at start, and logs out when the client closes stdin or on Ctrl-C / SIGTERM.

Claude Code:

```bash
claude mcp add passbolt -- passbolt-mcp --config /home/me/.config/passbolt-mcp/passbolt-mcp.toml
```

Claude Desktop, in `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "passbolt": {
      "command": "passbolt-mcp",
      "args": ["--config", "/home/me/.config/passbolt-mcp/passbolt-mcp.toml"]
    }
  }
}
```

## Over HTTP

For a client that cannot start a process, or several clients at once:

```bash
passbolt-mcp serve --http 127.0.0.1:8808
claude mcp add --transport http passbolt http://127.0.0.1:8808/mcp
```

Every client shares the session of the MCP user. `passbolt-mcp` listens on a loopback address only,
and refuses requests from a web page (those with an `Origin` header), so a page you visit cannot
reach it.

## With Docker

The image `ghcr.io/scadra/passbolt-mcp` serves over stdio. Put the configuration, the key and the
passphrase in one directory, with relative paths in the configuration, and mount it on `/config`:
writable, for the pinned server key and the audit file.

```bash
docker run --rm -v ~/.config/passbolt-mcp:/config ghcr.io/scadra/passbolt-mcp verify
claude mcp add passbolt -- docker run -i --rm -v ~/.config/passbolt-mcp:/config ghcr.io/scadra/passbolt-mcp
```

`--http` is not for the image: `passbolt-mcp` listens on a loopback address only, which a container
does not publish.

## Logs

On stderr, `warn` and up by default; `PASSBOLT_MCP_LOG=info` (or `debug`) for more. Secrets never
reach them.
