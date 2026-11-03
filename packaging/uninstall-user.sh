#!/usr/bin/env bash
set -euo pipefail
prefix="$HOME/.local/share"
if [ -n "$XDG_DATA_HOME" ]; then prefix="$XDG_DATA_HOME"; fi
rm -f "$HOME/.local/bin/addon-anvil" \
  "$prefix/applications/addon-anvil.desktop" \
  "$prefix/icons/hicolor/scalable/apps/addon-anvil.svg"
echo "Executable and desktop integration removed. Config and data preserved."
