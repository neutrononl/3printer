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

## Test

```sh
cargo fmt --all -- --check
cargo test --all-targets
```

## License

3printer is distributed under the CORE License.
See [LICENSE](LICENSE) for the full text.
