# mNFTS

A free, open-source, read-only NTFS driver for macOS 15.4+ built on Apple's FSKit framework.

## Status

**v0.1 Read-Only MVP**. mNFTS can mount NTFS volumes in read-only mode, allowing you to browse and copy files from NTFS-formatted drives on modern macOS.

## Architecture

mNFTS is composed of three layers:

- **libmnfts** (Rust core). Handles NTFS parsing (via the `ntfs` crate), block I/O, health checks, and exports a C-ABI for the Swift layer.
- **MNFTSFSKit** (Swift FSKit extension). A thin shell that bridges FSKit callbacks to the Rust core.
- **MNFTSCLI** (Swift CLI). The `mnfts` command-line tool for mounting, inspecting, and managing NTFS volumes.

Data flow: `Finder -> VFS -> fskitd -> MNFTSFSKit (Swift) -> libmnfts (Rust) -> ntfs crate -> block device`

## Installation

```bash
brew install mnfts    # coming soon
```

## Quick Start

```bash
mnfts mount /dev/disk4s1
```

## Commands

| Command | Description |
|---------|-------------|
| `mnfts mount <device>` | Mount an NTFS volume (read-only) |
| `mnfts unmount <mountpoint>` | Unmount a previously mounted volume |
| `mnfts emergency-unmount <mountpoint>` | Force-unmount a volume immediately |
| `mnfts status` | Show status of all mounted NTFS volumes |
| `mnfts inspect <device>` | Display NTFS metadata for a device |
| `mnfts doctor` | Run health checks on the driver and system |

## Known Limitations

- **Read-only**. Write support is not available in v0.1.
- **No compressed files**. NTFS-compressed files are not supported.
- **No encrypted files**. EFS-encrypted files are not supported.
- **macOS 15.4+ only**. FSKit APIs used by mNFTS require macOS Sequoia 15.4 or later.

## Building from Source

### Prerequisites

- Rust stable toolchain (install via [rustup](https://rustup.rs))
- Xcode 16+
- macOS 15.4+

### Build and Test

```bash
make build    # Build Rust core + Swift CLI
make test     # Run all tests
make lint     # Check formatting and clippy warnings
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for build instructions, coding standards, and PR requirements.

## License

MIT. See [LICENSE](LICENSE) for the full text.
