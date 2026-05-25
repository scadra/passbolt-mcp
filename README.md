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
| **The MCP rules, in `passbolt-mcp`** | whoever runs the MCP server, in its configuration | what the agent can **do**: which tools it gets, read only or write, secrets or not, which folders. Everything beyond reading is off by default. |

So:

- Removing a share in Passbolt cuts the agent off at once, without touching the MCP server.
- A rule can only take away: it never gives more than Passbolt does.
- Everything the agent does shows in Passbolt under the name of the MCP user, in the activity log
  and the history of each resource.
- Suspending or deleting the MCP user cuts everything.
