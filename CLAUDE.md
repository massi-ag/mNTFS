# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Build Commands

- `make build` - Build Rust core + Swift CLI (requires Rust toolchain + Xcode)
- `make test` - Run all tests
- `make lint` - Check formatting and clippy warnings
- `cargo test --test <name>` - Run a specific Rust test file
- `cargo test <test_fn>` - Run a single test by name

## Architecture

mNFTS is a read-only NTFS driver for macOS 15.4+ using FSKit.

- **crates/libmnfts/** - Rust core: NTFS parsing (via `ntfs` crate), block I/O, health checks, FFI exports
- **swift/Sources/MNFTSFSKit/** - Swift FSKit extension: thin shell calling libmnfts via C-ABI
- **swift/Sources/MNFTSCLI/** - Swift CLI: `mnfts mount/unmount/status/inspect/doctor`
- **swift/Sources/MNFTSWatchdog/** - Swift launchd agent: monitors FSKit extension, force-unmounts on crash

Data flow: Finder -> VFS -> fskitd -> MNFTSFSKit (Swift) -> libmnfts (Rust) -> ntfs crate -> block device

## Key Constraints

- Zero `unsafe` in Rust NTFS/journal/safety modules. `unsafe` only in `ffi/` and `io/` modules.
- v0.1 is read-only, no write code paths exist.
- Block device access via FSKit's provided resource handle, not direct `/dev/rdisk`.
- All tests require golden NTFS images in `tests/images/`. See `tests/images/README.md`.

## Testing

- `cargo test` runs all unit and integration tests
- Integration tests use NTFS disk images from `tests/images/`
- Test image paths use `env!("CARGO_MANIFEST_DIR")` for reliable resolution
