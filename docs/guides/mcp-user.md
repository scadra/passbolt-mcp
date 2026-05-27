---
title: The MCP user
sidebar_position: 2
---

# The MCP user

`passbolt-mcp` never logs in as a person. It logs in as a Passbolt user created for the agent: the
*MCP user*. Passbolt decides what that user can see; the [rules](../rules.md) decide what the agent
can do with it.

## Why a user of its own

- **Least privilege**: the agent sees only what is shared with the MCP user, not everything its
  operator can open.
- **Traceability**: every action shows in Passbolt under the MCP user, in the activity log and the
  history of each resource. You can tell what an agent did from what a person did.
- **Revocation**: removing a share, or suspending the user, cuts the agent off without touching
  anyone else's account or the MCP server.

Create one MCP user per agent, or per use (a CI job, a support assistant…), rather than one shared
by all of them.

## Create it

An administrator creates it, in the Passbolt web application:

1. **Users → Create**, with a name that says what it is (`MCP agent — CI`) and an e-mail address
   you can read: the invitation goes there.
2. Role **User**. Never **Admin**: `passbolt-mcp` refuses to run as an administrator.
3. Add it to a group such as `MCP agents`, so that sharing with every agent at once is one share.

## Activate it

The invitation is activated like any other, with the Passbolt browser extension, preferably in a
browser profile kept for this purpose:

1. Open the link of the invitation e-mail and follow the setup.
2. Choose a long passphrase, and keep it in a secret store: `passbolt-mcp` reads it from a file or a
   command, never from its command line.
3. Download the **recovery kit**: it is the private key of the MCP user (an armored `.asc` file).
4. Store the key where `passbolt-mcp` runs, readable by it only (`chmod 600`).

If your organisation enforces MFA, set up TOTP for the MCP user too, and keep its secret the same
way as the passphrase.

## Share with it

The MCP user sees nothing until something is shared with it.

- Make a folder for the agent (`Agents/CI`), put what it needs in it, and share the folder with
  the MCP user or its group.
- Give **Read** unless the agent must change the resources. **Update** lets it edit them;
  **Owner** lets it share and delete them: give it only when the agent must share.
- Keep the [rules](../rules.md) in line: a share with **Update** is useless while the rules keep
  the agent read only, and that is the safer default.

## Revoke

| To | Do, in Passbolt |
|---|---|
| Take one resource or folder away | remove the share with the MCP user (or its group) |
| Stop the agent altogether | suspend or delete the MCP user |
| Replace a leaked key or passphrase | delete the MCP user, create and activate a new one, share again |

None of these need the MCP server to restart: the next call of the agent sees the change.

## Check it

`passbolt-mcp check` logs in as the MCP user and prints who it is, its role, how many resources and
folders it can see, and what the rules let the agent do with them.
