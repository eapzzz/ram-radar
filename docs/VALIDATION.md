# Still 0.2 validation

Date: 2026-10-03. Local x86_64 Arch-family Linux, Hyprland, AMD Ryzen 5 5600, amdgpu Navi 23 / Radeon RX 6600 family, 19.45 GiB system memory and 8 GiB VRAM.

## Automated checks

- `cargo test --locked`: 17 tests passed. Includes stat parsing with parentheses, CPU guest-counter exclusion and counter resets, rate arithmetic, missing PSI, desktop parsing, executable collisions with populated desktop metadata, helper grouping, bounded history, missing-GPU CSV output, true timestamp chart spacing, and safe process termination rejection.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo fmt --check`: passed.
- `cargo build --release --locked`: passed.
- `desktop-file-validate still.desktop`: passed.
- `bash -n` for installer, uninstaller, launcher and PKGBUILD: passed.
- `makepkg --printsrcinfo`: passed; `.SRCINFO` generated. A clean-chroot package build and AUR submission are not claimed.
- Installer/uninstaller smoke test with a binary prefix containing spaces: desktop validation and `--version` passed, uninstall removed the installed binary.

## Live checks

The release binary ran on both native Wayland and XWayland. Screenshots show real system data, not fixtures. Overview, Applications, process details and Hardware were visually inspected; navigation and process expansion were exercised. GPU utilization, VRAM and the CPU model agree with local sysfs/procfs sources. CLI JSON was parsed and checked for sensible CPU/memory bounds and nonempty application/GPU records.

The desktop entry was installed to the current user's applications directory and validated with an absolute executable path. No root permissions or daemon installation are required.

## Independent review

An independent reviewer found misleading index-based chart spacing and clipping at maximum scale. Charts now use timestamps and explicit pause gaps; the content pane supports horizontal overflow and the search row wraps. A follow-up found basename-based grouping could merge unrelated executables; merging now requires a verified installation directory. A populated-registry regression checks both unrelated paths and valid application helpers.

## Boundaries

This verifies one AMD Linux machine. NVIDIA NVML, Intel GPU utilization, ARM runtime, remote filesystems and AUR installation are not verified or advertised. GPU absence remains an unavailable reading. No user process was terminated during UI testing; the automated safety test launches its own disposable `sleep` process and verifies a mismatched identity does not signal it.
