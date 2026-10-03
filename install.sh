#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
if [[ ${1:-} != --skip-build ]]; then cargo build --release --locked; fi
prefix=${PREFIX:-"$HOME/.local"}
data=${XDG_DATA_HOME:-"$HOME/.local/share"}
install -Dm755 target/release/still "$prefix/bin/still"
install -Dm644 assets/still.svg "$data/icons/hicolor/scalable/apps/still.svg"
# Desktop launchers need not inherit the shell PATH.
python3 - "$prefix/bin/still" "$data/applications/still.desktop" <<'PY'
import pathlib, sys
exe, destination = sys.argv[1:]
if any(c in exe for c in '\n\r'): raise SystemExit('Installation path cannot contain newlines')
def quoted(value):
    return '"' + value.replace('\\', '\\\\').replace('"', '\\"').replace('`', '\\`').replace('$', '\\$') + '"'
text = pathlib.Path('still.desktop').read_text()
text = '\n'.join('Exec=' + quoted(exe).replace('%', '%%') if line == 'Exec=still' else 'TryExec=' + exe if line == 'TryExec=still' else line for line in text.splitlines()) + '\n'
path = pathlib.Path(destination)
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(text)
PY
if command -v update-desktop-database >/dev/null; then update-desktop-database "$data/applications"; fi
printf 'Installed Still. Press Super and search for Still.\nBinary: %s/bin/still\n' "$prefix"
