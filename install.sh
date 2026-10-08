#!/bin/sh
# Installs zagzig-tui (the zagzig-tools terminal app) on Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/VladRafli/zagzig-tools/main/install.sh | sh
#
# Optional environment variables:
#   ZAGZIG_VERSION      version to install, like 0.14.0 or v0.14.0 (default: the latest release)
#   ZAGZIG_INSTALL_DIR  where to put the binary (default: $HOME/.local/bin)
#
# It downloads the release zip from GitHub, checks its SHA-256 against the
# release's checksums.txt, and copies the binary into place. Nothing needs root.
set -eu

REPO="VladRafli/zagzig-tools"
ASSET="zagzig-tui-x86_64-unknown-linux-gnu.zip"
INSTALL_DIR="${ZAGZIG_INSTALL_DIR:-$HOME/.local/bin}"

say() { printf '%s\n' "$*"; }
fail() { printf 'error: %s\n' "$*" >&2; exit 1; }

[ "$(uname -s)" = "Linux" ] || fail "this installer is for Linux. On Windows run: irm https://raw.githubusercontent.com/$REPO/main/install.ps1 | iex"
case "$(uname -m)" in
  x86_64 | amd64) ;;
  *) fail "no build for $(uname -m): releases only include x86_64 Linux." ;;
esac

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL --retry 3 -o "$2" "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -q -O "$2" "$1"; }
else
  fail "curl or wget is required."
fi

if command -v unzip >/dev/null 2>&1; then
  extract() { unzip -oq "$1" -d "$2"; }
elif command -v bsdtar >/dev/null 2>&1; then
  extract() { bsdtar -xf "$1" -C "$2"; }
elif command -v python3 >/dev/null 2>&1; then
  extract() { python3 -m zipfile -e "$1" "$2"; }
else
  fail "unzip is required (for example: sudo apt install unzip)."
fi

if command -v sha256sum >/dev/null 2>&1; then
  sha256() { sha256sum "$1" | cut -d ' ' -f 1; }
elif command -v shasum >/dev/null 2>&1; then
  sha256() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
else
  sha256() { return 1; }
fi

if [ -n "${ZAGZIG_VERSION:-}" ]; then
  version="${ZAGZIG_VERSION#v}"
  base="https://github.com/$REPO/releases/download/v$version"
  say "Installing zagzig-tui v$version"
else
  base="https://github.com/$REPO/releases/latest/download"
  say "Installing the latest zagzig-tui"
fi

tmp="$(mktemp -d)"
# Clean up on every exit, but keep the exit status (a bare `rm` would turn a failure into 0).
cleanup() { status=$?; rm -rf "$tmp"; exit "$status"; }
trap cleanup EXIT
trap 'exit 130' INT TERM

fetch "$base/$ASSET" "$tmp/$ASSET" || fail "couldn't download $base/$ASSET (does that version exist?)"

# The checksum is an integrity check (did the download complete), so a missing
# checksums.txt only produces a warning.
if fetch "$base/checksums.txt" "$tmp/checksums.txt" 2>/dev/null; then
  expected="$(grep " $ASSET\$" "$tmp/checksums.txt" | cut -d ' ' -f 1 | head -n 1)"
  actual="$(sha256 "$tmp/$ASSET" || true)"
  if [ -z "$expected" ]; then
    say "warning: checksums.txt has no entry for $ASSET, skipping the check."
  elif [ -z "$actual" ]; then
    say "warning: no sha256sum or shasum found, skipping the check."
  elif [ "$expected" != "$actual" ]; then
    fail "checksum mismatch for $ASSET (expected $expected, got $actual). Nothing was installed."
  else
    say "Checksum OK."
  fi
else
  say "warning: couldn't fetch checksums.txt, skipping the check."
fi

mkdir -p "$tmp/out" "$INSTALL_DIR"
extract "$tmp/$ASSET" "$tmp/out"
[ -f "$tmp/out/zagzig-tui" ] || fail "the archive doesn't contain zagzig-tui."
chmod +x "$tmp/out/zagzig-tui"
# Replace in two steps so a running copy isn't overwritten halfway.
cp "$tmp/out/zagzig-tui" "$INSTALL_DIR/zagzig-tui.new"
mv -f "$INSTALL_DIR/zagzig-tui.new" "$INSTALL_DIR/zagzig-tui"

say "Installed to $INSTALL_DIR/zagzig-tui"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) say "Run it with: zagzig-tui" ;;
  *) say "$INSTALL_DIR isn't on your PATH. Run it with: $INSTALL_DIR/zagzig-tui"
     say "To add it, put this in your shell profile: export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac
say "Raw ICMP pings need extra permission on Linux, see the manual's Linux notes if Connection Test reports permission denied."
