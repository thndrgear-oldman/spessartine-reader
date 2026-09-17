# Spessartine Reader

🚀 Blazingly fast ⚡, lightweight manga reader for Windows and Linux. Point it at your library
folders, pick a title, read. Everything works offline, with no accounts, ads
or telemetry.

## Screenshots

![Library](/pics/library_eng.png)

![Reading](/pics/pano_page_eng.png)

![End of chapter](/pics/end_of_chapter_eng.png)

## Features

- **Library** — scan folders, covers and thumbnails, chapters per title.
- **Reading modes** — Manga (single page), Double (two-page spread) and
  Webtoon (vertical column, smooth scrolling). Auto-detect the mode, or set
  it yourself for each title.
- **Reading direction** — right-to-left and left-to-right, detected
  automatically or set per title.
- **Progress** — where you stopped is saved and restored automatically.
  The next chapter is preloaded in the background.
- **Sound** — built-in page-flip sounds.
- **Archives** — reads ZIP/CBZ.
- **Interface** — English and Русский.

## Tech Stack

- **Rust** (edition 2024) — the whole app, no runtime
- **eframe / egui** with **egui_glow** — immediate-mode GUI
- **winit** — windowing, X11 / Wayland
- **rusqlite** (bundled SQLite) — library and reading progress
- **zip** — CBZ archive support
- **image** (JPEG/PNG/WebP) — decoding, dimensions read from headers only
- **kamadak-exif** — image metadata
- **rodio + symphonia** — page-flip sounds, ambient WAV/OGG
- **earcutr** — polygon tessellation for effect areas
- **TOML** — configuration files
- No network layer — anything and everything runs offline


## Build

Rust toolchain required (edition 2024). Linux needs the ALSA dev package:

```sh
# Debian / Ubuntu
sudo apt-get install libasound2-dev

# Arch / Manjaro
sudo pacman -S alsa-lib
```

Then:

```sh
cargo build --release
```

On Windows you may need the **Microsoft Visual C++ Redistributable** if
Windows reports `VCRUNTIME140.dll` missing.

## Controls

- **← / →** — previous / next page (two pages at once in Double mode)
- **Mouse wheel** — scroll in Webtoon mode
- **Right-click on a page** — reader menu
- **Right-click on a cover** — choose the reading mode for that title

## License

MIT · © 2026 thndrgear
