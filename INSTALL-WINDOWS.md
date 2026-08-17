# Installing Mycora on Windows

Two ways to get Mycora running, from least to most setup required.

## Option 1: prebuilt binary (recommended)

No Rust, no C compiler, nothing else to install.

1. Go to the [Releases page](https://github.com/TSODev/mycora/releases)
   and download `mycora-<version>-x86_64-pc-windows-msvc.zip` from the
   latest release.
2. Extract the `.zip` — it contains `mycora.exe` plus a copy of this
   repo's `README.md`/`USAGE.md`.
3. Open **Windows Terminal** (recommended — see [Terminal
   choice](#terminal-choice) below) and run it:

   ```powershell
   cd path\to\extracted\folder
   .\mycora.exe
   ```

That's it — Mycora creates its config and a starter vault on first run
(see [Where things live](#where-things-live) below).

If Windows refuses to launch `mycora.exe` with "The code execution
cannot proceed because VCRUNTIME140.dll was not found", you've got a
release built before 0.16.1 — the C++ runtime is statically linked into
the binary as of that release, so re-downloading the latest one fixes
it with nothing else to install. **Confirmed fixed on a real Windows 11
machine as of 0.16.1.** For an older release, installing the [Visual
C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)
works too.

To run `mycora` from any directory without typing the full path each
time, move `mycora.exe` somewhere already on your `PATH` (e.g.
`%USERPROFILE%\bin`, adding that folder to `PATH` via *Edit environment
variables for your account* if it isn't already), or add the extracted
folder to `PATH` directly.

## Option 2: `cargo install mycora`

Builds from source — needs both Rust and a working C compiler, since
Mycora's SQLite index (`rusqlite`'s `bundled` feature) compiles SQLite
from C source rather than depending on a system library.

1. Install Rust via [rustup](https://rustup.rs) (the installer defaults
   to the MSVC toolchain on Windows, which is what you want here).
2. Install the **Visual Studio Build Tools** — specifically the
   *Desktop development with C++* workload — from
   [visualstudio.microsoft.com/downloads](https://visualstudio.microsoft.com/downloads/)
   (scroll to "Tools for Visual Studio"). This is the C compiler/linker
   step; skipping it is the single most common reason `cargo install`
   fails partway through on a fresh Windows machine, with a linker error
   rather than anything mentioning SQLite directly.
3. From a terminal:

   ```powershell
   cargo install mycora
   mycora
   ```

## Where things live

Config, session state, and the disposable search index all live under
`%APPDATA%\mycora\` (Windows' standard per-app data folder, `dirs`
crate's `config_dir()`/`data_dir()` — both resolve to the same place on
Windows, unlike the separate `~/.config`/`~/.local/share` split on
Linux):

- `%APPDATA%\mycora\config.toml` — the vault registry and language
  setting
- `%APPDATA%\mycora\session.toml` — remembered selection/expand state
- `%APPDATA%\mycora\index.sqlite3` — the search index (safe to delete
  any time; `mycora reindex` rebuilds it)

The default vault itself (your actual notes) is created at
`%USERPROFILE%\mycora\` — inside your user profile folder, not the
app-data one, so it's easy to find, back up, or put under version
control on its own.

## Terminal choice

Mycora renders through `crossterm`, which works in any Windows
terminal, but **Windows Terminal** (preinstalled on Windows 11, a free
Microsoft Store install on Windows 10) gives the most reliable results
for the box-drawing characters and Unicode Mycora's panes and icons use
(`▾`/`▸`/`⊘`/`▦`, backlinks/tag markers). The legacy `cmd.exe` console
host can render these incorrectly or substitute `?` depending on the
configured font and code page.

## This is new — please report issues

Windows support is new as of this writing: the code paths above
(`dirs`-based config/data/vault-path resolution) work correctly on
Linux and macOS today, and have been reasoned through carefully for
Windows, but are only lightly exercised so far on a real Windows
machine by the people building Mycora. The missing-`VCRUNTIME140.dll`
issue above was the first real-world report — fixed in 0.16.1 and
**confirmed working on a real Windows 11 machine**. If something above
doesn't work as described, please [open an
issue](https://github.com/TSODev/mycora/issues) — Windows-specific bug
reports are especially useful right now.
