# 3printer

3printer is a desktop 3D-printing workspace for importing models, arranging
them on a build plate, generating experimental toolpaths, and reviewing
G-code before export.

## Features

- Import STL, OBJ, and 3MF models.
- Import additional scene formats on macOS through the native Swift importer.
- Move, rotate, scale, and center models on the build plate.
- Configure printer profiles and persist them locally.
- Generate and preview experimental Marlin G-code.
- Export printer settings, projects, and G-code.

3printer is an independent rebuild with a basic local slicer. Cloud services,
network printing, model repair, and the original advanced slicing engine are
not connected. Inspect generated G-code carefully before using it with
hardware.

## Requirements

- Rust and Cargo for building from source.
- Python 3 for 3MF importing.
- Swift on macOS for the additional native scene formats.

## Build and run

```sh
cargo run
```

To build the application without launching it:

```sh
cargo build --release
```

The release binary is created at `target/release/3printer` (or
`target/release/3printer.exe` on Windows).

On macOS, package a Finder-launchable application with the project icon:

```sh
./scripts/package-macos.sh
open dist/3printer.app
```

This creates `dist/3printer.app`. It can be opened by double-clicking it or
dragged into `/Applications`. The package is unsigned, so macOS may ask for
confirmation the first time it is opened.

On Linux, create a clickable desktop launcher:

```sh
./scripts/package-linux.sh
```

This creates `dist/3printer-linux/3printer.desktop`. Open that launcher from a
file manager to start 3printer without a terminal.

On Windows, run PowerShell from the project directory:

```powershell
.\scripts\package-windows.ps1
```

This creates `dist\3printer-windows\3printer.exe`. The Windows build uses the
GUI subsystem, so double-clicking it does not open a console window.

GitHub Actions builds all three packages automatically when a non-prerelease
GitHub Release is published, and attaches the archives to that Release.

## Test

```sh
cargo fmt --all -- --check
cargo test --all-targets
```

## License

3printer is distributed under the CORE License.
See [LICENSE](LICENSE) for the full text.
