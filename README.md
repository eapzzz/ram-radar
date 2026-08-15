<div align="center">

# 🎯 RamRadar
### High-Tech Linux RAM & Process Tree HUD for Arch Linux & Hyprland

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![GUI](https://img.shields.io/badge/GUI-egui%20%2F%20eframe-blue.svg?style=for-the-badge)](https://github.com/emilk/egui)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20Wayland%20%7C%20X11-darkgreen.svg?style=for-the-badge&logo=linux)](https://archlinux.org/)
[![Optimized for](https://img.shields.io/badge/Compositor-Hyprland-teal.svg?style=for-the-badge&logo=hyprland)](https://hyprland.org/)
[![License](https://img.shields.io/badge/License-MIT-purple.svg?style=for-the-badge)](LICENSE)

*An ultra-fast, modern memory inspector and HUD visualizer that reveals the truth about your RAM consumption using Proportional Set Size (PSS) metrics.*

[Features](#-key-features) • [Why PSS?](#-why-ramradar-pss-vs-rss) • [Installation](#-installation--building) • [Usage](#-usage) • [Hyprland Setup](#-hyprland-desktop-integration) • [Architecture](#-project-architecture)

---

</div>

## 💡 Why RamRadar? (PSS vs RSS)

Most traditional Linux task managers (such as standard `ps`, `top`, `htop`, or basic desktop monitors) display memory usage using **RSS** (*Resident Set Size*).

> **The RSS Problem:** RSS duplicates shared memory (`.so` libraries, IPC buffers, font caches) across every single child process. When modern browsers (Chromium, Firefox) or apps (Discord, Spotify, IDEs) spawn 20+ subprocesses, shared libraries are counted 20 times over, presenting a distorted, artificially inflated RAM consumption.

```
                  ┌────────────────────────────────────────────────────────┐
Traditional RSS   │ [App Private Memory] + [Shared Libs (Counted 100%)]   │ -> Lies & overcounts!
                  └────────────────────────────────────────────────────────┘
                  ┌────────────────────────────────────────────────────────┐
Real PSS (Radar)  │ [App Private (USS)] + [Shared Libs / Sharing Procs]   │ -> 100% Accurate!
                  └────────────────────────────────────────────────────────┘
```

**RamRadar delivers total accuracy:**
- Parses `/proc/[pid]/smaps_rollup` at multi-core speeds.
- Proportional memory distribution formula:
  $$\text{PSS} = \text{Private Memory (USS)} + \sum \frac{\text{Shared Memory Segment}}{\text{Number of Processes Sharing Segment}}$$
- Shows **the exact amount of physical memory reclaimed** if an application is closed.

---

## ✨ Key Features

- 🎯 **Interactive Sonar Radar Visualizer** — Real-time 60 FPS sweeping radar with phosphor trails, expanding sonar pulses, and target blips plotted dynamically in polar coordinates according to memory weight.
- 🧠 **Accurate PSS Memory Metrics** — No more phantom RAM numbers; exact allocation tracking for every process.
- 🌲 **Smart Multi-Process Tree Grouping** — Automatically consolidates multi-threaded trees (Chromium, Discord, Spotify, VS Code, Antigravity IDE, Game processes) into clean, expandable application cards.
- 🎨 **Futuristic Obsidian & Cyber Glass Theme** — Deep dark obsidian palette (`#07090E`), glowing neon accents, and smooth animations optimized for Wayland and Hyprland.
- ⚡ **Parallel Multi-Core Engine (Rayon)** — Concurrent scanning of hundreds of processes from `/proc` in sub-millisecond execution times.
- 📊 **Bandwidth Visualizer & Live Oscilloscope** — Segmented multi-color memory band paired with a real-time spline waveform trendline.
- 🏷️ **Intelligent System Categorization**:
  - 🌐 *User Applications* (Web Browsers, Media, Messengers)
  - 🪟 *Desktop & Wayland Stack* (Hyprland, Waybar, SwayNC, Rofi, Pipewire, Portals)
  - 💻 *Dev Tools & CLI* (Terminals, Compilers, Cargo, Git, Runtimes)
  - ⚙️ *Background Services* (Daemons, Agents, Helpers)
  - 🛡️ *System & Kernel Core* (systemd, udev, kworkers, D-Bus)
- 🔍 **Instant Search & Multi-Column Sorting** — Real-time filtering by name, role, or PID with instant sorting across PSS, RSS, USS, CPU %, or Process Count.
- 🛑 **Process Management & Signal Dispatch** — Terminate individual subprocesses or whole application trees (`SIGTERM`) directly from the HUD.
- 🖥️ **Terminal Summary (CLI Mode)** — Built-in ASCII dashboard for instant terminal inspections without GUI overhead.

---

## 🖥️ Terminal Mode (CLI Summary)

Want a fast terminal report without launching the GUI? Use the built-in CLI flag:

```bash
ram-radar --cli
```

#### Sample Output:
```text
===============================================================
🎯 RamRadar — Realistic Linux RAM Consumption (PSS Metrics)
===============================================================

Total RAM: 31.12 GB | System Used: 8.45 GB | Sum PSS: 6.20 GB (19.9%)
Available: 22.67 GB | Swap Used:   0 KB / 8.00 GB

APPLICATION / GROUP              PROCS      REAL (PSS)      TRADITIONAL (RSS) % RAM   
--------------------------------------------------------------------------------------
🌐 Google Chrome                 18 procs   2.10 GB         5.80 GB           6.7%
⚡ Antigravity IDE               8 procs    890.0 MB        1.60 GB           2.8%
💬 Discord                       6 procs    620.4 MB        1.10 GB           2.0%
🎵 Spotify Music                 4 procs    380.2 MB        650.0 MB          1.2%
🪟 Hyprland Compositor & Shell   1 procs    320.1 MB        410.2 MB          1.0%
💻 Terminals & Shells (CLI)      3 procs    140.5 MB        210.0 MB          0.4%
```

---

## 🚀 Installation & Building

### Prerequisites:
- **Rust Toolchain** (`rustup` / `cargo`):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Wayland / System Graphics Libraries** (e.g., on Arch Linux):
  ```bash
  sudo pacman -S --needed base-devel wayland libxkbcommon
  ```

### Clone & Build:

```bash
git clone https://github.com/eapzzz/ram-radar.git
cd ram-radar

# Compile optimized release binary with LTO and binary stripping
cargo build --release
```

The optimized binary is compiled to `target/release/ram-radar`.

---

## 🎮 Usage

### Launching the Graphical HUD:
```bash
./run.sh
# or directly:
./target/release/ram-radar
```

### CLI Terminal Mode:
```bash
./target/release/ram-radar --cli
```

---

## 🪟 Hyprland Desktop Integration

### Keybind Configuration:
Add the following line to your Hyprland configuration (`~/.config/hypr/hyprland.conf`) to launch RamRadar via hotkey (e.g., `$mainMod + Shift + ESC`):

```ini
bind = $mainMod SHIFT, Escape, exec, ram-radar
```

### Floating Window Rules:
To render RamRadar as a centered, translucent floating HUD panel:

```ini
windowrulev2 = float, class:^(ram-radar)$
windowrulev2 = size 1180 800, class:^(ram-radar)$
windowrulev2 = center, class:^(ram-radar)$
windowrulev2 = opacity 0.96 0.92, class:^(ram-radar)$
```

### Desktop Application Launcher (`.desktop`):
The launcher and the keybind above both call `ram-radar` by name, so put the
binary on your `PATH` first, then install the desktop entry:

```bash
install -Dm755 target/release/ram-radar ~/.local/bin/ram-radar

cp ram-radar.desktop ~/.local/share/applications/
update-desktop-database ~/.local/share/applications/ 2>/dev/null || true
```

---

## 🏗️ Project Architecture

```text
ram-radar/
├── Cargo.toml                  # Dependencies & LTO release profile
├── ram-radar.desktop           # Linux Desktop launcher entry
├── run.sh                      # Helper run script
├── README.md                   # Documentation
└── src/
    ├── main.rs                 # Entry point (CLI argument parser & eframe native loop)
    ├── process/
    │   ├── mod.rs
    │   ├── types.rs            # Core models: ProcessInfo, AppGroup, Category, SystemMemoryInfo
    │   ├── scanner.rs          # Rayon parallel /proc scanner (smaps_rollup, stat, meminfo)
    │   └── classifier.rs       # Rule-based application grouping & role heuristics
    └── ui/
        ├── mod.rs
        ├── app.rs              # 60 FPS egui state loop, async scanner channel, event handling
        ├── theme.rs            # Obsidian glass palette, glowing primitives, typography
        ├── animation.rs        # Radar sweep angle, pulse waves, EMA metric smoothers
        └── components/
            ├── radar_view.rs       # Custom Sonar Radar visualizer with range rings & target blips
            ├── header.rs           # Live pulse HUD header, filter pills, search & sort
            ├── hero_metrics.rs     # Radar gadget & 4 telemetry metric cards
            ├── memory_visualizer.rs# Stacked bandwidth band & live spline oscilloscope
            └── app_card.rs         # Expandable group cards, process roles & kill actions
```

---

## 📜 License

Distributed under the **MIT License**. See [LICENSE](LICENSE) for details.
