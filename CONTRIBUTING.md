# Contributing to Still

Keep measurements understandable and the monitor inexpensive. New collectors must run off the UI thread, represent unsupported readings explicitly, and include parser/delta tests. Do not add per-application name lists, network services, or unconditional frame loops.

Before opening a PR:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release --locked
desktop-file-validate still.desktop
```

Test changes against both readable and permission-denied procfs entries. Resource totals and process estimates are different measurements. Never promise that PSS is the amount freed by terminating a process.

For UI changes include a screenshot at the default size and at 850 × 620. Explain the actual environment used; label fixture screenshots. Do not include private command lines or personal paths.
