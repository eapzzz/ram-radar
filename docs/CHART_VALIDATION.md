# Chart hover and history ranges

Validated on 2026-10-06 on local x86_64 Linux, KDE Wayland, with the app running through XWayland at 1.0 interface scale.

## Checks

- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo test --locked`: 29 tests passed. New checks cover hour-long retention, timestamp-based window filtering, full-history CSV export, paused-window anchoring, saved custom ranges, legacy settings, custom bounds, named percent/rate hover values, nearest-sample precision, collection gaps, unrecorded time, and singleton/gap-edge marker selection.
- `cargo build --release --locked`: passed.
- `desktop-file-validate still.desktop` and `git diff --check`: passed.
- Independent review found an edge case when hovering the lone point in a one-second window. Two failing egui hover regressions reproduced it; a four-pixel marker tolerance fixed it, and the follow-up review reported no remaining important issues.

The system Rust 1.99 compiler failed before compiling project code with an unresolved LLVM symbol. Checks used the locally cached Rust 1.98.1 and matching LLVM 22 libraries extracted under `/tmp`, without changing system packages or project dependencies.

## Visual checks

The History page was captured and inspected at [1280 × 880](screenshots/history-range.png) with the one-minute preset and [850 × 620](screenshots/history-range-compact.png) with Custom set to 3600 seconds. Both screenshots contain live system measurements, with isolated temporary preferences and exports. Controls fit at both sizes; smaller windows use the existing scroll area for content below the viewport.

Tooltip content and sample selection were exercised using the real chart widget in egui frames with pointer input. These checks do not depend on desktop pointer automation. The live screenshots document the layout and range controls, rather than a hovered tooltip.

Hour retention was exercised with timestamped test samples; no hour-long wall-clock observation is claimed. A selected window remains anchored to the latest collected sample while paused. Earlier unrecorded time and collection gaps show no invented values, and exporting CSV keeps the full retained history regardless of the selected display window.
