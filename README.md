	      ____                  __          ____
	     / __ \____  _____ ____/ /_  ____  / / /_
	    / /_/ / __ `/ ___/ ___/ __ \/ __ \/ / __/
	   / ____/ /_/ (__  |__  ) /_/ / /_/ / / /_
	  /_/    \__,_/____/____/_.___/\____/_/\__/

	Open source password manager for teams
	(c) 2026 Passbolt SA
	https://www.passbolt.com

## License

Passbolt - Open source password manager for teams

(c) 2026 Passbolt SA

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General
Public License (AGPL) as published by the Free Software Foundation version 3.

The name "Passbolt" is a registered trademark of Passbolt SA, and Passbolt SA hereby declines to grant a trademark
license to "Passbolt" pursuant to the GNU Affero General Public License version 3 Section 7(e), without a separate
agreement with Passbolt SA.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied
warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program. If not,
see [GNU Affero General Public License v3](http://www.gnu.org/licenses/agpl-3.0.html).

## About passbolt-mcp

An [MCP](https://modelcontextprotocol.io) server that lets an AI agent (Claude Code, Claude Desktop,
any MCP client) work with [Passbolt](https://www.passbolt.com) directly: find a resource, read a
password, rotate it, share a folder.

The agent never uses the account of a person. It works through **a Passbolt user of its own**, the
*MCP user*: an ordinary Passbolt user, created by an administrator, with its own OpenPGP private
key. `passbolt-mcp` logs in with that key (GPGAuth, as the browser extension does) and turns the
agent's requests into Passbolt API calls.

### Two barriers

What the agent can do is limited twice, and it only gets what **both** allow:

```
  agent ──MCP──▶ passbolt-mcp ──────────────▶ Passbolt
                 the MCP rules                the MCP user
                 what it may DO               what it may SEE
```

| Barrier | Set by | Limits |
|---|---|---|
| **The MCP user, in Passbolt** | the administrator and the owners of the resources, in Passbolt | what the agent can **see**: only the resources and folders shared with that user (or its group), with the permission given (read, update, owner). Its role is `user`, never `admin`. |
| **The MCP rules, in `passbolt-mcp`** | whoever runs the MCP server, in its configuration | what the agent can **do**: which tools it gets, read only or write, secrets or not, which folders, how often; which calls a person must approve. Everything beyond reading is off by default. |

So:

- Removing a share in Passbolt cuts the agent off at once, without touching the MCP server.
- A rule can only take away: it never gives more than Passbolt does.
- Everything the agent does shows in Passbolt under the name of the MCP user, in the activity log
  and the history of each resource.
- Suspending or deleting the MCP user cuts everything.

On top of the two barriers, a **person** can stay in the loop: with `[confirm]`, the tools listed
(`get_secret`, the deletes, the shares…) run only after the person says yes in the client, and the
agent cannot answer for them. Every call is written to an audit file, never a secret.

### What the agent can do

- Find resources and folders, and read their name, username, URIs, description, comments and
  permissions.
- With the rules allowing it:
  - **secrets**: read a password or a note, get a TOTP code (never its seed), check a password
    against Have I Been Pwned, generate one following the password policy of the organization; or, with `mask_secrets`, **use** a password without
    seeing it: it gets a handle, and `passbolt-mcp` puts the password in a declared command, API
    call or file, or shows it to the person only;
  - **writing**: create, update, move and delete resources and folders, comment, mark favorites,
    each with a `dry_run`;
  - **sharing**: find users and groups, simulate a share, share and revoke.

31 tools in all, plus MCP resources and prompts: see the [tools](docs/tools.md).

### What it cannot do

- See anything not shared with the MCP user, or outside the folders the rules allow.
- Do more than the permission Passbolt gives the MCP user: a resource shared as **Read** stays read
  only, whatever the rules say.
- Act as an administrator: `passbolt-mcp` refuses to run as one.
- Go faster than `[limits]` allow, in calls or secrets per minute.

### Example

An administrator creates the user `MCP agent — CI`, shares the folder `Agents/CI` with it as **Read**,
and the operator of the agent allows secrets in that folder only:

```toml
[access]
secrets = true

[scope]
folders = ["Agents/CI"]
```

Asked "deploy with the staging database password", the agent finds `Staging DB` in `Agents/CI`
and reads its password. Asked to read the production password, kept elsewhere, it finds nothing;
asked to change `Staging DB`, it has no tool to do so.

## Install

From a [release](https://github.com/scadra/passbolt-mcp/releases): a binary for Linux, macOS and
Windows, or the Docker image `ghcr.io/scadra/passbolt-mcp`. From the sources, with Rust 1.88 or
later:

```bash
cargo install --locked --git https://github.com/scadra/passbolt-mcp
```

Then, with the [MCP user](docs/guides/mcp-user.md) created and its [rules](docs/rules.md) written:

```bash
passbolt-mcp verify   # pin the server key, once
passbolt-mcp check    # who the agent will be, and what it can do
claude mcp add passbolt -- passbolt-mcp --config ~/.config/passbolt-mcp/passbolt-mcp.toml
```

Claude Desktop, Cursor, VS Code, Codex, Docker, HTTP: see [connect a client](docs/guides/serve.md).

## Documentation

The documentation is published at **https://scadra.github.io/passbolt-mcp/**, from
[`docs/`](docs/index.mdx):

- **Get started**: [what is passbolt-mcp](docs/index.mdx), [installation](docs/getting-started/installation.md),
  [quick start](docs/getting-started/quick-start.md)
- **Learn**: [architecture](docs/architecture.md), [the MCP user](docs/guides/mcp-user.md),
  [rules](docs/rules.md), [tools](docs/tools.md), [example prompts](docs/examples.md)
- **Use**: [connect a client](docs/guides/serve.md), [rule recipes](docs/guides/recipes.md),
  [troubleshooting](docs/troubleshooting.md)
- **Security**: [security best practices](docs/security.md),
  [use a secret without seeing it](docs/secrets-without-seeing.md)
- **Reference**: [tools reference](docs/tools-reference.md), [changelog](CHANGELOG.md)
- **Project**: [roadmap](docs/roadmap.md), [development](docs/contributing/development.md)

## Tests

Every tool, every rule, `[confirm]` and HTTP are tested against a real Passbolt server, through a
real MCP client running the binary (`scripts/e2e.sh`); unit tests and a fake Passbolt cover the
rest on each pull request. See [development](docs/contributing/development.md).

## Reporting a security issue

If you've found a security related issue in Passbolt, please don't open an issue on GitHub. Follow our responsible disclosure process: https://www.passbolt.com/docs/contribute/security/vulnerability/.

# Credits

https://www.passbolt.com/credits
