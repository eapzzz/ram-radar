# Still design

Rebuild RamRadar as a useful native Linux system monitor, launched from the desktop. The user authorizes a complete redesign and renaming, with an approximately one-hour budget and remaining weekly usage as a hard resource constraint.

Rust / egui remain: no web server, Electron, background daemon or cloud service. A background sampler reads procfs/sysfs, publishing only the latest snapshot. UI redraws on input or samples. Default sampling is 2 seconds; history is bounded to 10 minutes. Missing sensors are unavailable, never invented zeros. PSS estimates are labeled. CPU process percentages use one logical core = 100%.

Overview shows resource usage, a compact time-series and top consumers. Applications is a searchable, sortable inventory with real desktop icons, grouping by executable identity, process details and explicit-confirmation termination. Hardware provides CPU cores, GPU, VRAM, storage, network and hwmon sensors. History shows session trends and CSV export. Settings retain sampling and appearance preferences locally.

Visual direction: ink navy #151B26, panel #1D2532, text #EDF1F7, muted #A2AFC1, blue #8CB7F5, peach #F1B995. Restrained horizontal resource meters; a clear left navigation rail, aligned tabular numbers, no radar or ornament. Default bundled sans with optional system font. App icon is an original stacked signal mark.

Linux sources: /proc/stat, meminfo, pressure, net/dev, diskstats, self/mountinfo, /sys/class/drm, hwmon and power_supply. No commands assembled from process data. Process actions verify PID start time. Exports exclude process command lines. Packaging includes desktop entry, icon, local install/uninstall scripts, source PKGBUILD, CI, license and accurate README.

Validation: parser and delta tests, grouping collision tests, cargo test / clippy / fmt, release build, real system CLI, live GUI screenshots and interactions, desktop entry validation. GPU support follows what each kernel driver exports; document limitations.
