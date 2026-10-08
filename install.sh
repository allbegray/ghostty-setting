#!/bin/sh
# Install or remove ghostty-setting.
#
#   curl -fsSL https://raw.githubusercontent.com/allbegray/ghostty-setting/main/install.sh | sh
#   curl -fsSL https://raw.githubusercontent.com/allbegray/ghostty-setting/main/install.sh | sh -s -- --uninstall
#
# Install downloads the prebuilt binary for this machine when the release has
# one, and falls back to building from source with cargo when it does not.
# Remove takes back whichever of the two this script would have created.
#
# Environment:
#   PREFIX   where the binary lives (default: ~/.local/bin)

set -eu

REPO="allbegray/ghostty-setting"
PREFIX="${PREFIX:-$HOME/.local/bin}"
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
BINARY="ghostty-setting"

say() { printf '%s\n' "$*" >&2; }
die() { say "error: $*"; exit 1; }

usage() {
    say "usage: install.sh [--uninstall]"
    say ""
    say "  (no argument)   install into $PREFIX"
    say "  --uninstall     remove the installed binary"
    say ""
    say "PREFIX=$PREFIX"
}

# Remove whichever copy exists — the one this script downloads and the one
# cargo would have installed. The Ghostty config is never touched: that file is
# the user's, and the app only ever edited it because it was asked to.
uninstall() {
    removed=0

    if [ -f "$PREFIX/$BINARY" ]; then
        rm -f "$PREFIX/$BINARY"
        say "removed $PREFIX/$BINARY"
        removed=1
    fi

    if [ -f "$CARGO_BIN/$BINARY" ] && command -v cargo >/dev/null 2>&1; then
        if cargo uninstall "$BINARY" >/dev/null 2>&1; then
            say "removed $CARGO_BIN/$BINARY"
            removed=1
        fi
    fi

    if [ "$removed" -eq 0 ]; then
        say "nothing to remove: no $BINARY in $PREFIX or $CARGO_BIN"
    fi

    say "your Ghostty configuration was left alone"
}

case "${1:-}" in
    --uninstall|uninstall) uninstall; exit 0 ;;
    -h|--help) usage; exit 0 ;;
    "") ;;
    *) die "unknown argument: $1 (try --help)" ;;
esac

os=$(uname -s)
arch=$(uname -m)

case "$os" in
    Darwin) ;;
    *)
        say "note: this installer handles macOS; on $os build from source instead:"
        say "  cargo install --git https://github.com/$REPO --locked"
        exit 1
        ;;
esac

case "$arch" in
    arm64) target="aarch64-apple-darwin" ;;
    x86_64) target="x86_64-apple-darwin" ;;
    *) die "unsupported architecture: $arch" ;;
esac

asset="$BINARY-$target"
url="https://github.com/$REPO/releases/latest/download/$asset"

mkdir -p "$PREFIX"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

say "looking for a prebuilt binary: $asset"
if curl -fsSL "$url" -o "$tmp/$BINARY" 2>/dev/null; then
    install -m 0755 "$tmp/$BINARY" "$PREFIX/$BINARY"
    say "installed $PREFIX/$BINARY"
else
    say "no prebuilt binary for $target yet; building from source instead"
    command -v cargo >/dev/null 2>&1 ||
        die "cargo not found — install Rust from https://rustup.rs and re-run, or build from a checkout"
    cargo install --git "https://github.com/$REPO" --locked
    say "installed with cargo"
fi

case ":$PATH:" in
    *":$PREFIX:"*) ;;
    *)
        say ""
        say "add $PREFIX to your PATH, then restart your shell:"
        say "  export PATH=\"$PREFIX:\$PATH\""
        ;;
esac

say ""
say "run: $BINARY"
say "remove: curl -fsSL https://raw.githubusercontent.com/$REPO/main/install.sh | sh -s -- --uninstall"
