#!/usr/bin/env bash
set -euo pipefail
prefix=${PREFIX:-"$HOME/.local"}
data=${XDG_DATA_HOME:-"$HOME/.local/share"}
python3 - "$prefix" "$data" <<'PY'
from pathlib import Path
import sys
prefix, data = map(Path, sys.argv[1:])
for path in [prefix/'bin/still', data/'applications/still.desktop', data/'icons/hicolor/scalable/apps/still.svg']:
    path.unlink(missing_ok=True)
PY
if command -v update-desktop-database >/dev/null; then update-desktop-database "$data/applications"; fi
printf 'Removed Still. Preferences and exported CSV files are preserved.\n'
