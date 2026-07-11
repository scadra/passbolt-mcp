---
title: Connect a client
sidebar_position: 3
---

# Connect a client

Once the [MCP user](mcp-user.md) exists, its key and passphrase are in place, the
[rules](../rules.md) are written and the server is verified:

```bash
passbolt-mcp verify   # once
passbolt-mcp check    # who the agent will be, and what it can do
```

The client starts `passbolt-mcp` and talks to it on stdin and stdout. It logs in as the MCP user
at start, and logs out when the client closes stdin or on Ctrl-C / SIGTERM. Without `--config`, it
reads `passbolt-mcp.toml` in the configuration directory of the system
(`~/.config/passbolt-mcp/` on Linux); clients do not expand `~`, so write absolute paths.

Most clients take the same entry, under the key their file names (`mcpServers`, `servers`…):

```json
{
  "command": "passbolt-mcp",
  "args": ["--config", "/home/me/.config/passbolt-mcp/passbolt-mcp.toml"]
}
```

## Claude Code

```bash
claude mcp add --scope user passbolt -- passbolt-mcp --config /home/me/.config/passbolt-mcp/passbolt-mcp.toml
```

`--scope user` for every project; without it, the current one only. In a project shared with
others, `.mcp.json` holds `{"mcpServers": {"passbolt": …}}` with the entry above. `/mcp` in Claude
Code shows it connected, and its tools.

## Claude Desktop

`claude_desktop_config.json`: Settings → Developer → Edit Config, or
`~/Library/Application Support/Claude/` on macOS, `%APPDATA%\Claude\` on Windows.

```json
{
  "mcpServers": {
    "passbolt": {
      "command": "passbolt-mcp",
      "args": ["--config", "/Users/me/.config/passbolt-mcp/passbolt-mcp.toml"]
    }
  }
}
```

Restart Claude Desktop; `passbolt` shows in the tools menu of a conversation.

## Cursor

`~/.cursor/mcp.json`, or `.cursor/mcp.json` in a project:

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

## VS Code

`.vscode/mcp.json` in a project, or **MCP: Open User Configuration**; the key is `servers`:

```json
{
  "servers": {
    "passbolt": {
      "type": "stdio",
      "command": "passbolt-mcp",
      "args": ["--config", "/home/me/.config/passbolt-mcp/passbolt-mcp.toml"]
    }
  }
}
```

## Codex

`~/.codex/config.toml`:

```toml
[mcp_servers.passbolt]
command = "passbolt-mcp"
args = ["--config", "/home/me/.config/passbolt-mcp/passbolt-mcp.toml"]
```

## Gemini CLI

`~/.gemini/settings.json`, or `.gemini/settings.json` in a project:

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

## Zed

`settings.json`, under `context_servers`:

```json
{
  "context_servers": {
    "passbolt": {
      "command": "passbolt-mcp",
      "args": ["--config", "/home/me/.config/passbolt-mcp/passbolt-mcp.toml"]
    }
  }
}
```

## Other clients

Any client that starts an MCP server over stdio: the command `passbolt-mcp`, the arguments
`--config <path>`. Or the environment variable `PASSBOLT_MCP_CONFIG` instead of `--config`.

## With Docker

The image `ghcr.io/scadra/passbolt-mcp` serves over stdio. Put the configuration, the key and the
passphrase in one directory, with relative paths in the configuration, and mount it on `/config`:
writable, for the pinned server key and the audit file.

```bash
docker run --rm -v ~/.config/passbolt-mcp:/config ghcr.io/scadra/passbolt-mcp verify
claude mcp add passbolt -- docker run -i --rm -v ~/.config/passbolt-mcp:/config ghcr.io/scadra/passbolt-mcp
```

In a JSON file, the same, with an absolute path:

```json
{
  "mcpServers": {
    "passbolt": {
      "command": "docker",
      "args": ["run", "-i", "--rm", "-v", "/home/me/.config/passbolt-mcp:/config",
               "ghcr.io/scadra/passbolt-mcp:0.1.0"]
    }
  }
}
```

`-i`, never `-t`: a terminal mangles the MCP messages. Pin a version (`:0.1.0`) rather than
`latest`, to choose when to upgrade.

## Over HTTP

For a client that cannot start a process, or several clients at once:

```bash
passbolt-mcp serve --http 127.0.0.1:8808
claude mcp add --transport http passbolt http://127.0.0.1:8808/mcp
```

Every client shares the session of the MCP user. `passbolt-mcp` listens on a loopback address only,
and refuses requests from a web page (those with an `Origin` header), so a page you visit cannot
reach it. For the same reason, `--http` is not for the Docker image: a container does not publish
its loopback address.

## Logs

On stderr, `warn` and up by default; `PASSBOLT_MCP_LOG=info` (or `debug`) for more. Secrets never
reach them. When the client does not connect: [troubleshooting](../troubleshooting.md).
