# Change Log
All notable changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]
### Added
- `[confirm]`: the person approves the tools listed before they run, through the client (MCP
  elicitation: `input_required` from protocol 2026-07-28, `elicitation/create` before); a client
  that cannot ask is refused, or allowed with `unsupported = "allow"`. `whoami` and `check` list
  them; the audit file writes `asked` and `declined`
- Tests against a real Passbolt server for every tool, the rules, `[confirm]` and HTTP
- The roadmap, in the documentation
- The documentation follows the layout and style of the MCP documentation: get started, learn, use,
  security, reference; an architecture overview; security best practices attack by attack; a page
  on using a secret without seeing it
- `access.mask_secrets`: `get_secret` and `generate_password` return a handle in place of the
  value, which `create_resource` and `update_resource` take
- `run_with_secret` and `[commands]`: `passbolt-mcp` runs a command of the configuration with a
  password in its environment, and gives back its output with the password taken out
- `call_api` and `[apis]`: a request to an API of the configuration, the secret in a header, the
  path kept under its base URL, redirects not followed, the answer scrubbed
- `fill_template` and `[templates]`: a file of the configuration written from a template the
  agent wrote, its `{{ passbolt:… }}` replaced with secrets, readable by its owner only
- `show_secret`: shows a secret to the person, in the client, never to the agent

### Changed
- `generate_password` follows the password policy of the organization, as the browser
  extension does: password or passphrase, length, character sets; PIN codes (`kind`); it says
  the strength, and whether it is below the policy. `lowercase`, `uppercase` and `digits` are
  gone: `symbols = false` gives letters and digits
- `passbolt-sdk` 0.1.6

### Fixed
- Builds with Rust 1.99, which refuses a `schemars` attribute before its derive
- `delete_folder` without `with_content` says what Passbolt does: the content moves to the root,
  not to the parent

## [0.1.0] - 2026-07-09
### Added
- `passbolt-mcp serve`: an MCP server, over stdio or Streamable HTTP (`--http`, a loopback address
  only, requests with an `Origin` refused), on `passbolt-sdk` and `rmcp`
- The MCP user: a Passbolt user of its own for the agent, logged in with its OpenPGP private key
  (GPGAuth), its passphrase from a file or a command, TOTP when the server asks for it; logged in
  again when the session expires, logged out on exit; administrators refused
- `passbolt-mcp verify`, to pin the server key before the first login, and `passbolt-mcp check`,
  to tell who the agent will be and what it can do, with warnings on risky rules
- The MCP rules, in `passbolt-mcp.toml`: `[access]` (secrets, write, share, `allow_tools`,
  `deny_tools`), `[scope]` (the folders the agent works in), `[audit]`, `[limits]`; read only by
  default, and a rule never gives more than Passbolt does
- Reading tools: `whoami`, `list_resources`, `get_resource`, `list_folders`, `folder_tree`,
  `list_comments`, `get_permissions`, `list_resource_types`; lists in pages
- Secret tools: `get_secret`, `get_totp_code`, `check_password_pwned` (k-anonymity),
  `generate_password`; secrets never reach the logs or the errors
- Writing tools, with `dry_run`: `create_resource`, `update_resource`, `delete_resource`,
  `move_resource`, `create_folder`, `rename_folder`, `move_folder`, `delete_folder`,
  `set_favorite`, `add_comment`, `delete_comment`; refused before reaching Passbolt when the
  permission of the MCP user does not allow them
- Sharing tools: `search_aros`, `simulate_share`, `share_resource`, `share_folder`; moves give
  the permissions of the new folder when the rules allow sharing
- MCP resources `passbolt://resource/{id}` and `passbolt://folder/{id}`, and the prompts
  `audit_password_hygiene` and `rotate_password`
- Audit file (JSON lines, mode 0600) of every call, without secrets; limits on calls and secrets
  per minute
- Names, descriptions and comments marked as untrusted data, hidden characters removed
- Documentation in `docs/`, with a tools reference written from the schemas
- Tests against a fake Passbolt (the one of the SDK tests) and, with `scripts/e2e.sh`, against a
  real server
- Release binaries for Linux, macOS and Windows, and the Docker image `ghcr.io/scadra/passbolt-mcp`

[Unreleased]: https://github.com/scadra/passbolt-mcp/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/scadra/passbolt-mcp/releases/tag/v0.1.0
