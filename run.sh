#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
if [[ ! -x target/release/still ]]; then cargo build --release --locked; fi
exec ./target/release/still "$@"
