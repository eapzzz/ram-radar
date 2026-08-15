<div align="center">

# 🎯 RamRadar
### Realistic Linux RAM & Process Tree Inspector for Arch Linux & Hyprland

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![GUI](https://img.shields.io/badge/GUI-egui%20%2F%20eframe-blue.svg?style=for-the-badge)](https://github.com/emilk/egui)
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20Wayland%20%7C%20X11-darkgreen.svg?style=for-the-badge&logo=linux)](https://archlinux.org/)
[![Optimized for](https://img.shields.io/badge/Compositor-Hyprland-teal.svg?style=for-the-badge&logo=hyprland)](https://hyprland.org/)
[![License](https://img.shields.io/badge/License-MIT-purple.svg?style=for-the-badge)](LICENSE)

*An ultra-fast, modern memory and process inspector that tells the truth about your RAM consumption using Proportional Set Size (PSS) metrics.*

[Funkcje](#-funkcje--features) • [Dlaczego PSS?](#-dlaczego-ramradar-pss-vs-rss) • [Instalacja](#-instalacja--budowanie) • [Uruchomienie](#-u%C5%BCycie) • [Integracja z Hyprland](#-integracja-ze-%C5%9Brodowiskiem-hyprland)

---

</div>

## 💡 Dlaczego RamRadar? (PSS vs RSS)

Większość tradycyjnych menedżerów procesów na Linuksie (np. podstawowy `ps`, `top`, starsze narzędzia systemowe) prezentuje zużycie pamięci w oparciu o wskaźnik **RSS** (Resident Set Size).

> **Problem z RSS:** RSS sumuje współdzielone biblioteki (shared memory / `.so`) dla każdego procesu z osobna. Jeśli przeglądarka (np. Chromium) uruchomi 20 procesów potomnych, pamięć współdzielona zostanie zliczona 20 razy, pokazując zawyżone, nierealne zużycie RAM.

**RamRadar rozwiązuje ten problem:**
- Odczytuje metryki **PSS** (*Proportional Set Size*) z `/proc/[pid]/smaps_rollup`.
- Pamięć współdzielona jest dzielona proporcjonalnie między wszystkie procesy, które z niej korzystają:
  $$\text{PSS} = \text{Pamięć Prywatna (USS)} + \frac{\text{Pamięć Współdzielona}}{\text{Liczba Procesów Współdzielących}}$$
- Daje to **100% realistyczny obraz** tego, ile pamięci rzeczywiście zwalnia zamknięcie danej aplikacji.

---

## ✨ Funkcje / Key Features

- 🧠 **Realistyczna Analityka Pamięci (PSS)** — Koniec z zakłamanymi wynikami RSS; precyzyjna alokacja każdego megabajta.
- 🌲 **Inteligentne Grupowanie Procesów** — Automatyczne łączenie procesów wielowątkowych/drzew (Chromium, Discord, Spotify, VS Code, IDE, deemony systemowe) w pojedyncze, przejrzyste karty aplikacji.
- 🎨 **Nowoczesny Interfejs Dark / Glassmorphism** — Zaprojektowany w `egui` pod ciemne motywy, perfekcyjnie komponujący się ze środowiskami Wayland / Hyprland.
- ⚡ **Równoległe Skanowanie (Rayon)** — Błyskawiczny odczyt setek procesów z `/proc` na wielu rdzeniach CPU bez obciążania systemu.
- 📊 **Wizualizator Paskowy & Hero Metrics** — Przegląd pamięci całkowitej, dostępnej, używanej przez aplikacje, środowisko graficzne, narzędzia deweloperskie i system.
- 🏷️ **Kategoryzacja Systemowa**:
  - 🌐 *Aplikacje Użytkownika* (Przeglądarki, Komunikatory, Odtwarzacze)
  - 🪟 *Środowisko Hyprland & Desktop* (Hyprland, Waybar, SwayNC, Rofi, Pipewire)
  - 💻 *Narzędzia Developerskie & Terminale* (Kitty, Alacritty, IDE, Kompilatory, Git)
  - ⚙️ *Usługi Tła Użytkownika* (1Password, Daemony, Agenty)
  - 🛡️ *System i Jądro* (systemd, udev, sterowniki)
- 🔍 **Wyszukiwarka i Zaawansowane Sortowanie** — Błyskawiczne filtrowanie procesów w czasie rzeczywistym oraz sortowanie wg PSS, RSS, CPU, PID lub Nazwy.
- 🛑 **Zarządzanie Procesami** — Bezpieczne wysyłanie sygnałów zakończenia (`SIGTERM` / `SIGKILL`) bezpośrednio z GUI.
- 💻 **Tryb Terminalowy (CLI Mode)** — Szybki raport ASCII w terminalu za pomocą flagi `--cli` / `--summary` / `-c`.

---

## 🖥️ Tryb CLI / Terminal Summary

Nie chcesz otwierać GUI? RamRadar posiada wbudowany, szybki tryb CLI:

```bash
ram-radar --cli
```

Przykładowy zrzut z terminala:
```text
===============================================================
🎯 RamRadar — Realistyczne Zużycie Pamięci RAM (PSS Metrics)
===============================================================

Pamięć całkowita: 31.12 GB | Używany (System): 8.45 GB | Suma PSS procesów: 6.20 GB (19.9%)

[🌐 Aplikacje Użytkownika] - 4.12 GB (13.2% RAM)
  • Google Chrome (18 proc.)                PSS: 2.10 GB | RSS: 5.80 GB | CPU: 4.2%
  • Discord (6 proc.)                       PSS: 620.4 MB | RSS: 1.10 GB | CPU: 1.1%
  • Spotify (4 proc.)                       PSS: 380.2 MB | RSS: 650.0 MB | CPU: 0.5%

[🪟 Środowisko Hyprland & Desktop] - 850.3 MB (2.7% RAM)
  • Hyprland (1 proc.)                      PSS: 320.1 MB | RSS: 410.2 MB | CPU: 2.0%
  • Waybar (1 proc.)                        PSS: 95.4 MB | RSS: 140.0 MB | CPU: 0.2%
  • SwayNC (1 proc.)                        PSS: 78.2 MB | RSS: 110.5 MB | CPU: 0.0%

[💻 Narzędzia Developerskie & Terminale] - 1.15 GB (3.7% RAM)
  • Antigravity IDE (8 proc.)               PSS: 890.0 MB | RSS: 1.60 GB | CPU: 3.5%
  • Kitty (2 proc.)                         PSS: 140.5 MB | RSS: 210.0 MB | CPU: 0.8%
```

---

## 🚀 Instalacja & Budowanie

### Wymagania wstępne:
- Zainstalowany kompilator Rust & Cargo (`rustup`):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- Standardowe pakiety graficzne / Wayland (np. na Arch Linux):
  ```bash
  sudo pacman -S --needed base-devel wayland libxkbcommon
  ```

### Klonowanie i kompilacja:

```bash
git clone git@github.com:eapzzz/ram-radar.git
cd ram-radar

# Kompilacja zoptymalizowanego wydania release (LTO + Strip)
cargo build --release
```

Gotowy plik wykonywalny znajdziesz w `target/release/ram-radar`.

---

## 🎮 Użycie

### Uruchomienie aplikacji GUI:
```bash
./run.sh
# lub bezpośrednio:
./target/release/ram-radar
```

### Tryb podsumowania w konsoli:
```bash
./target/release/ram-radar --cli
```

---

## 🪟 Integracja ze środowiskiem Hyprland

### Skrót klawiszowy w `hyprland.conf`:
Dodaj poniższą regułę do konfiguracji Hyprland (`~/.config/hypr/hyprland.conf`), aby uruchamiać RamRadar dedykowanym skrótem (np. `$mainMod + Shift + ESC`):

```ini
bind = $mainMod SHIFT, Escape, exec, /home/julian/Documents/cookie/projects/ram-radar/target/release/ram-radar
```

### Reguły okna (Floating Window):
Aby RamRadar otwierał się jako wycentrowane, pływające okno w stylu panelu monitorowania:

```ini
windowrulev2 = float, class:^(ram-radar)$
windowrulev2 = size 1150 780, class:^(ram-radar)$
windowrulev2 = center, class:^(ram-radar)$
```

### Aktywator Desktop (.desktop):
Plik `ram-radar.desktop` jest gotowy do umieszczenia w `~/.local/share/applications/`:

```bash
cp ram-radar.desktop ~/.local/share/applications/
update-desktop-database ~/.local/share/applications/ 2>/dev/null || true
```
Od tego momentu aplikacja będzie widoczna w Rofi, Wofi, Walkerze, ToFi i innych launcherach.

---

## 🏗️ Architektura Projektu

```text
ram-radar/
├── Cargo.toml                  # Zależności i konfiguracja wydania LTO
├── ram-radar.desktop           # Integracja z menu aplikacji Linuksa
├── run.sh                      # Skrypt uruchamiający
└── src/
    ├── main.rs                 # Punkt wejścia (obsługa flag CLI & eframe native)
    ├── process/
    │   ├── mod.rs
    │   ├── types.rs            # Modele danych: ProcessInfo, AppGroup, Category, SystemMemoryInfo
    │   ├── scanner.rs          # Wielowątkowy skaner /proc (smaps_rollup, statm, meminfo)
    │   └── classifier.rs       # Inteligentna kategoryzacja procesów wg reguł
    └── ui/
        ├── mod.rs
        ├── app.rs              # Główna pętla stanu egui & renderowania
        ├── theme.rs            # Paleta barw (Glassmorphism / Neon Dark) & widgety
        ├── animation.rs        # Płynne interpolacje stanów
        ├── process_tree.rs     # Renderowanie drzew i wierszy procesów
        ├── stacked_bar.rs      # Pasek proporcji zużycia pamięci
        └── components/
            ├── header.rs           # Pasek nagłówka (wyszukiwanie, sortowanie, odświeżanie)
            ├── hero_metrics.rs     # Karty kluczowych wskaźników (Total, PSS, Available, Swap)
            ├── memory_visualizer.rs # Wizualizacja rozkładu kategorii
            └── app_card.rs         # Karta grupy aplikacji (rozwijana lista procesów)
```

---

## 📜 Licencja

Projekt udostępniany na licencji **MIT**. Szczegóły w pliku [LICENSE](LICENSE).
