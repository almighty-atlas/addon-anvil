#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release
prefix="$HOME/.local/share"
if [ -n "${XDG_DATA_HOME:-}" ]; then prefix="$XDG_DATA_HOME"; fi
bin="$HOME/.local/bin"
install -Dm755 target/release/addon-anvil "$bin/addon-anvil"
install -Dm644 packaging/addon-anvil.desktop "$prefix/applications/addon-anvil.desktop"
install -Dm644 assets/icons/addon-anvil.svg "$prefix/icons/hicolor/scalable/apps/addon-anvil.svg"
echo "Installed: $bin/addon-anvil"
echo "Ensure ~/.local/bin is in PATH"
