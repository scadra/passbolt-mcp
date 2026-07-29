#!/usr/bin/env bash
# Print the Homebrew formula of a release, from the .sha256 files of its archives.
# Usage: scripts/homebrew-formula.sh <version> <directory of the .sha256 files>
set -euo pipefail

version="$1"
dir="$2"
base="https://github.com/scadra/passbolt-mcp/releases/download/v$version"

sha() {
  cut -d ' ' -f 1 "$dir/passbolt-mcp-v$version-$1.tar.gz.sha256"
}

archive() {
  printf '      url "%s/passbolt-mcp-v%s-%s.tar.gz"\n      sha256 "%s"\n' \
    "$base" "$version" "$1" "$(sha "$1")"
}

cat <<EOF
class PassboltMcp < Formula
  desc "MCP server: an AI agent works with Passbolt through a user of its own"
  homepage "https://scadra.github.io/passbolt-mcp/"
  version "$version"
  license "AGPL-3.0-or-later"

  on_macos do
    on_arm do
$(archive aarch64-apple-darwin)
    end
    on_intel do
$(archive x86_64-apple-darwin)
    end
  end

  on_linux do
    on_arm do
$(archive aarch64-unknown-linux-musl)
    end
    on_intel do
$(archive x86_64-unknown-linux-musl)
    end
  end

  def install
    bin.install "passbolt-mcp"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/passbolt-mcp --version")
  end
end
EOF
