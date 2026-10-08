#!/bin/sh
# Install ghostty-setting.
#
#   curl -fsSL https://raw.githubusercontent.com/allbegray/ghostty-setting/main/install.sh | sh
#
# Downloads the prebuilt binary for this machine when the release has one, and
# falls back to building from source with cargo when it does not.
#
# Environment:
#   PREFIX   where to put the binary (default: ~/.local/bin)

set -eu

REPO="allbegray/ghostty-setting"
PREFIX="${PREFIX:-$HOME/.local/bin}"

say() { printf '%s\n' "$*" >&2; }
die() { say "error: $*"; exit 1; }

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

asset="ghostty-setting-$target"
url="https://github.com/$REPO/releases/latest/download/$asset"

mkdir -p "$PREFIX"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

say "looking for a prebuilt binary: $asset"
if curl -fsSL "$url" -o "$tmp/ghostty-setting" 2>/dev/null; then
    install -m 0755 "$tmp/ghostty-setting" "$PREFIX/ghostty-setting"
    say "installed $PREFIX/ghostty-setting"
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
say "run: ghostty-setting"
