#!/usr/bin/env bash
# Run the end-to-end tests against a real Passbolt server, as an MCP user (role user).
# Settings: PASSBOLT_MCP_E2E_* from the environment or .env.e2e (see .env.e2e.example).
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ -f .env.e2e ]]; then
  set -a
  source .env.e2e
  set +a
fi
: "${PASSBOLT_MCP_E2E_SERVER_URL:?}" "${PASSBOLT_MCP_E2E_PRIVATE_KEY_FILE:?}"
: "${PASSBOLT_MCP_E2E_PASSPHRASE_FILE:?}" "${PASSBOLT_MCP_E2E_SHARE_WITH:?}"

dir="$(mktemp -d)"
trap 'rm -rf "$dir"' EXIT
config="$dir/passbolt-mcp.toml"
{
  echo "[passbolt]"
  echo "server_url = \"$PASSBOLT_MCP_E2E_SERVER_URL\""
  echo "private_key_file = \"$(realpath "$PASSBOLT_MCP_E2E_PRIVATE_KEY_FILE")\""
  echo "passphrase_file = \"$(realpath "$PASSBOLT_MCP_E2E_PASSPHRASE_FILE")\""
  echo "timeout_seconds = ${PASSBOLT_MCP_E2E_TIMEOUT:-240}"
  if [[ -n "${PASSBOLT_MCP_E2E_RESOLVE:-}" ]]; then
    echo "resolve = [\"$PASSBOLT_MCP_E2E_RESOLVE\"]"
  fi
  if [[ "${PASSBOLT_MCP_E2E_TLS_SKIP_VERIFY:-false}" == "true" ]]; then
    echo "tls_skip_verify = true"
  fi
} > "$config"
# The pinned server key, or the one the server sends now.
if [[ -n "${PASSBOLT_MCP_E2E_SERVER_KEY_FILE:-}" ]]; then
  cp "$PASSBOLT_MCP_E2E_SERVER_KEY_FILE" "$dir/server-key.asc"
else
  cargo run --release --quiet -- --config "$config" verify
fi
# Release: OpenPGP is much faster optimized. Tests create their own folders, so they run in parallel.
PASSBOLT_MCP_E2E_CONFIG="$config" \
  cargo test --release "$@" -- --ignored --test-threads="${E2E_THREADS:-2}"
