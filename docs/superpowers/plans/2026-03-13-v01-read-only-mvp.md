# mNFTS v0.1 Read-Only MVP — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a read-only NTFS driver for macOS 15.4+ that mounts external drives via FSKit, exposes files in Finder, and installs via Homebrew.

**Architecture:** Rust core library (`libmnfts`) linked via C-ABI FFI into a Swift FSKit system extension. A Swift CLI (`mnfts`) controls mount/unmount via FSKit management APIs. A Swift watchdog launchd agent monitors the extension process and force-unmounts on crash.

**Tech Stack:** Rust (core library, `ntfs` crate, `tracing`), Swift (FSKit extension, CLI via `ArgumentParser`, watchdog), `cbindgen` (FFI headers), macOS 15.4+ FSKit framework.

**Spec:** `docs/superpowers/specs/2026-03-13-mnfts-design.md`

---

## File Structure

### Rust Workspace (`crates/`)

| File | Responsibility |
|---|---|
| `Cargo.toml` (workspace root) | Workspace definition, shared dependencies |
| `crates/libmnfts/Cargo.toml` | Core library crate config |
| `crates/libmnfts/src/lib.rs` | Public API surface, re-exports |
| `crates/libmnfts/src/io/mod.rs` | Block I/O trait + types |
| `crates/libmnfts/src/io/block_device.rs` | Block device reader (read-only, file descriptor based) |
| `crates/libmnfts/src/io/cache.rs` | LRU block cache |
| `crates/libmnfts/src/volume/mod.rs` | NTFS volume mounting, boot sector validation, volume flags |
| `crates/libmnfts/src/volume/boot_sector.rs` | Boot sector parsing and validation |
| `crates/libmnfts/src/volume/health.rs` | Volume health checks (dirty flag, hibernation, NTFS version) |
| `crates/libmnfts/src/fs/mod.rs` | Filesystem operations (readdir, read file, stat) |
| `crates/libmnfts/src/fs/dir.rs` | Directory enumeration via index B-trees |
| `crates/libmnfts/src/fs/file.rs` | File reading via data runs |
| `crates/libmnfts/src/fs/attrs.rs` | Attribute resolution ($STANDARD_INFORMATION, $FILE_NAME, $DATA) |
| `crates/libmnfts/src/fs/metadata.rs` | Timestamp conversion, permissions mapping, file ID generation |
| `crates/libmnfts/src/inspect.rs` | Volume metadata dump for `mnfts inspect` |
| `crates/libmnfts/src/doctor.rs` | Health check report for `mnfts doctor` |
| `crates/libmnfts/src/ffi/mod.rs` | C-ABI exports for Swift FFI |
| `crates/libmnfts/src/ffi/types.rs` | FFI-safe types (repr(C) structs for cross-boundary data) |
| `crates/libmnfts/src/error.rs` | Error types |
| `crates/libmnfts/build.rs` | cbindgen header generation |
| `crates/libmnfts/cbindgen.toml` | cbindgen config |
| `crates/libmnfts/tests/` | Integration tests directory |
| `crates/libmnfts/tests/mount_and_read.rs` | Integration tests: mount image, enumerate, read files |
| `crates/libmnfts/tests/health_checks.rs` | Integration tests: dirty, hibernated, corrupt volumes |
| `crates/libmnfts/tests/fixtures/` | Test NTFS disk images (small, checked in) |

### Swift Package (`swift/`)

| File | Responsibility |
|---|---|
| `swift/Package.swift` | Swift package manifest (FSKit module, CLI, Watchdog targets) |
| `swift/Sources/MNFTSFSKit/MNFTSFileSystem.swift` | FSKit filesystem subclass — delegates all ops to libmnfts FFI |
| `swift/Sources/MNFTSFSKit/FFIBridge.swift` | Swift wrapper around C-ABI functions from libmnfts |
| `swift/Sources/MNFTSFSKit/ItemMapping.swift` | FSItemID ↔ MFT reference mapping |
| `swift/Sources/MNFTSFSKit/Info.plist` | Extension metadata, FSKit registration |
| `swift/Sources/MNFTSCLI/main.swift` | CLI entry point |
| `swift/Sources/MNFTSCLI/Commands/MountCommand.swift` | `mnfts mount` command |
| `swift/Sources/MNFTSCLI/Commands/UnmountCommand.swift` | `mnfts unmount` command |
| `swift/Sources/MNFTSCLI/Commands/StatusCommand.swift` | `mnfts status` command |
| `swift/Sources/MNFTSCLI/Commands/InspectCommand.swift` | `mnfts inspect` command |
| `swift/Sources/MNFTSCLI/Commands/DoctorCommand.swift` | `mnfts doctor` command |
| `swift/Sources/MNFTSWatchdog/main.swift` | Watchdog launchd agent — monitors FSKit extension via kqueue |
| `swift/Sources/MNFTSWatchdog/ProcessMonitor.swift` | kqueue/EVFILT_PROC based process watcher |
| `swift/Sources/MNFTSWatchdog/CrashReporter.swift` | Write crash reports to ~/Library/Logs/mNFTS/ |
| `swift/Sources/MNFTSWatchdog/com.mnfts.watchdog.plist` | launchd agent plist |

### Root Files

| File | Responsibility |
|---|---|
| `Makefile` | Orchestrates Rust build + Swift build + linking + install |
| `README.md` | Project overview, install instructions, quick start |
| `LICENSE` | MIT license |
| `CONTRIBUTING.md` | Build instructions, test guide, contribution tiers |
| `CLAUDE.md` | Claude Code project instructions |
| `.github/workflows/ci.yml` | CI: build, test, lint, fuzz |
| `.github/SECURITY.md` | Security disclosure policy |
| `.github/ISSUE_TEMPLATE/bug_report.md` | Bug report template |
| `Formula/mnfts.rb` | Homebrew formula |

---

## Chunk 1: Project Scaffolding & FSKit Proof of Concept (Phase 0)

This chunk establishes the build system, proves FSKit works, and proves Rust-Swift FFI works. No NTFS code yet.

### Task 1: Initialize Rust Workspace

**Files:**
- Create: `Cargo.toml`
- Create: `crates/libmnfts/Cargo.toml`
- Create: `crates/libmnfts/src/lib.rs`
- Create: `crates/libmnfts/src/error.rs`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
[workspace]
members = ["crates/libmnfts"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
```

- [ ] **Step 2: Create libmnfts crate with dependencies**

```toml
[package]
name = "libmnfts"
version.workspace = true
edition.workspace = true
license.workspace = true

[lib]
crate-type = ["staticlib", "lib"]

[dependencies]
ntfs = "0.4"
thiserror = "2"
tracing = "0.1"

[build-dependencies]
cbindgen = "0.28"

[dev-dependencies]
tracing-subscriber = "0.3"
```

- [ ] **Step 3: Create error types**

```rust
// crates/libmnfts/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MnftsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("NTFS error: {0}")]
    Ntfs(#[from] ntfs::NtfsError),

    #[error("Volume is not NTFS")]
    NotNtfs,

    #[error("Volume is dirty — mount read-only or run chkdsk on Windows")]
    DirtyVolume,

    #[error("Volume has Windows hibernation active — cannot mount")]
    Hibernated,

    #[error("Unsupported NTFS version: {0}.{1}")]
    UnsupportedVersion(u8, u8),

    #[error("Volume health check failed: {0}")]
    HealthCheck(String),
}

pub type Result<T> = std::result::Result<T, MnftsError>;
```

- [ ] **Step 4: Create lib.rs with module stubs**

```rust
// crates/libmnfts/src/lib.rs
pub mod error;

pub use error::{MnftsError, Result};
```

- [ ] **Step 5: Verify build**

Run: `cargo build`
Expected: Compiles with no errors.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/
git commit -m "chore: initialize Rust workspace with libmnfts crate"
```

---

### Task 2: Set Up cbindgen for C-ABI Header Generation

**Files:**
- Create: `crates/libmnfts/cbindgen.toml`
- Create: `crates/libmnfts/build.rs`
- Create: `crates/libmnfts/src/ffi/mod.rs`
- Create: `crates/libmnfts/src/ffi/types.rs`
- Modify: `crates/libmnfts/src/lib.rs`

- [ ] **Step 1: Create cbindgen config**

```toml
# crates/libmnfts/cbindgen.toml
language = "C"
header = "/* Generated by cbindgen — do not edit */"
include_guard = "LIBMNFTS_H"
autogen_warning = "/* Warning: this file is autogenerated by cbindgen. Don't modify this manually. */"

[export]
prefix = "mnfts_"

[enum]
rename_variants = "ScreamingSnakeCase"
```

- [ ] **Step 2: Create build.rs**

```rust
// crates/libmnfts/build.rs
fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let config = cbindgen::Config::from_file("cbindgen.toml")
        .expect("Unable to find cbindgen.toml");

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("include/libmnfts.h");
}
```

- [ ] **Step 3: Create FFI types**

```rust
// crates/libmnfts/src/ffi/types.rs

/// Result code returned by all FFI functions.
#[repr(C)]
pub enum MnftsResult {
    Ok = 0,
    ErrIo = 1,
    ErrNotNtfs = 2,
    ErrDirtyVolume = 3,
    ErrHibernated = 4,
    ErrUnsupportedVersion = 5,
    ErrHealthCheck = 6,
    ErrNullPointer = 7,
    ErrInternal = 99,
}

/// Opaque handle to a mounted NTFS volume.
pub struct MnftsVolume {
    _private: [u8; 0],
}
```

- [ ] **Step 4: Create FFI module with a test function**

```rust
// crates/libmnfts/src/ffi/mod.rs
pub mod types;

use types::MnftsResult;

/// Returns MnftsResult::Ok. Used to verify FFI linkage works.
#[no_mangle]
pub extern "C" fn mnfts_ping() -> MnftsResult {
    MnftsResult::Ok
}

/// Returns the library version as a static C string.
#[no_mangle]
pub extern "C" fn mnfts_version() -> *const std::ffi::c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const std::ffi::c_char
}
```

- [ ] **Step 5: Update lib.rs to include ffi module**

```rust
// crates/libmnfts/src/lib.rs
pub mod error;
pub mod ffi;

pub use error::{MnftsError, Result};
```

- [ ] **Step 6: Build and verify header generation**

Run: `cargo build`
Expected: `crates/libmnfts/include/libmnfts.h` is generated with `mnfts_ping` and `mnfts_version` declarations.

Run: `cat crates/libmnfts/include/libmnfts.h`
Expected: Contains `MnftsResult mnfts_ping(void);` and `const char *mnfts_version(void);`

- [ ] **Step 7: Commit**

```bash
git add crates/libmnfts/cbindgen.toml crates/libmnfts/build.rs crates/libmnfts/src/ffi/ crates/libmnfts/include/
git commit -m "feat: add cbindgen FFI header generation with ping/version exports"
```

---

### Task 3: Initialize Swift Package with FSKit Extension Stub

**Files:**
- Create: `swift/Package.swift`
- Create: `swift/Sources/MNFTSFSKit/MNFTSFileSystem.swift`
- Create: `swift/Sources/MNFTSFSKit/FFIBridge.swift`

- [ ] **Step 1: Create Swift package manifest**

```swift
// swift/Package.swift
// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "mNFTS",
    platforms: [.macOS(.v15)],
    products: [
        .executable(name: "mnfts", targets: ["MNFTSCLI"]),
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-argument-parser.git", from: "1.5.0"),
    ],
    targets: [
        .systemLibrary(
            name: "CLibMNFTS",
            path: "Sources/CLibMNFTS",
            pkgConfig: nil,
            providers: nil
        ),
        .target(
            name: "MNFTSFSKit",
            dependencies: ["CLibMNFTS"],
            linkerSettings: [
                .linkedLibrary("mnfts", .when(platforms: [.macOS])),
                .unsafeFlags(["-L../../target/release"], .when(platforms: [.macOS])),
            ]
        ),
        .executableTarget(
            name: "MNFTSCLI",
            dependencies: [
                .product(name: "ArgumentParser", package: "swift-argument-parser"),
            ]
        ),
    ]
)
```

- [ ] **Step 2: Create C module map for libmnfts**

Create `swift/Sources/CLibMNFTS/module.modulemap`:
```
module CLibMNFTS {
    header "../../../crates/libmnfts/include/libmnfts.h"
    link "mnfts"
    export *
}
```

Create `swift/Sources/CLibMNFTS/shim.h` (empty shim required by SPM):
```c
// Shim header — actual declarations in libmnfts.h via module map
```

- [ ] **Step 3: Create FFI bridge**

```swift
// swift/Sources/MNFTSFSKit/FFIBridge.swift
import CLibMNFTS
import Foundation

enum MNFTSBridge {
    static func ping() -> Bool {
        return mnfts_ping() == MNFTS_RESULT_OK
    }

    static func version() -> String {
        guard let cStr = mnfts_version() else { return "unknown" }
        return String(cString: cStr)
    }
}
```

- [ ] **Step 4: Create FSKit filesystem stub**

```swift
// swift/Sources/MNFTSFSKit/MNFTSFileSystem.swift
import FSKit
import Foundation

/// Stub FSKit filesystem — to be confirmed in Phase 0 whether
/// FSUnaryFileSystem or FSBlockDeviceFileSystem is correct.
/// For now, start with the simplest subclass that compiles.
final class MNFTSFileSystem: FSUnaryFileSystem {
    override func load(
        resource: FSResource,
        options: FSTaskOptions
    ) async throws -> FSUnaryFileSystem.LoadResult {
        // Phase 0 stub: validate that FSKit loads us
        let volumeName = "NTFS-Test"
        return LoadResult(volumeName: volumeName, status: .ready)
    }
}
```

Note: The exact FSKit API may differ — Phase 0 exists to discover the real API surface. This stub is a starting point to verify compilation and linkage.

- [ ] **Step 5: Create minimal CLI entry point**

```swift
// swift/Sources/MNFTSCLI/main.swift
import ArgumentParser

@main
struct MNFTS: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "mnfts",
        abstract: "Mount and manage NTFS volumes on macOS",
        version: "0.1.0",
        subcommands: [VersionCmd.self]
    )
}

struct VersionCmd: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "version",
        abstract: "Print version information"
    )

    func run() {
        print("mnfts 0.1.0")
    }
}
```

- [ ] **Step 6: Build Rust library first, then Swift package**

Run: `cargo build --release`
Run: `cd swift && swift build`
Expected: Both compile. Swift links against the Rust static library. CLI binary runs: `swift run mnfts version` prints "mnfts 0.1.0".

- [ ] **Step 7: Commit**

```bash
git add swift/
git commit -m "feat: initialize Swift package with FSKit stub, CLI, and FFI bridge"
```

---

### Task 4: Create Makefile for Unified Build

**Files:**
- Create: `Makefile`

- [ ] **Step 1: Create Makefile**

```makefile
# Makefile — mNFTS build orchestration

.PHONY: all build build-rust build-swift clean test test-rust test-swift lint

RUST_TARGET_DIR := target
SWIFT_BUILD_DIR := swift/.build

all: build

build: build-rust build-swift

build-rust:
	cargo build --release

build-swift: build-rust
	cd swift && swift build -c release

clean:
	cargo clean
	cd swift && swift package clean

test: test-rust test-swift

test-rust:
	cargo test

test-swift: build-rust
	cd swift && swift test

lint:
	cargo fmt -- --check
	cargo clippy -- -D warnings

# Install CLI binary to /usr/local/bin
install: build
	cp swift/.build/release/mnfts /usr/local/bin/mnfts

uninstall:
	rm -f /usr/local/bin/mnfts
```

- [ ] **Step 2: Verify full build**

Run: `make build`
Expected: Rust compiles, Swift compiles, no errors.

Run: `make test`
Expected: All tests pass (currently just default tests).

- [ ] **Step 3: Commit**

```bash
git add Makefile
git commit -m "chore: add Makefile for unified Rust + Swift build"
```

---

### Task 5: FSKit Proof of Concept — Mount Dummy Filesystem

**Files:**
- Modify: `swift/Sources/MNFTSFSKit/MNFTSFileSystem.swift`
- Create: `swift/Sources/MNFTSFSKit/Info.plist`

This is the most exploratory task in the entire plan. FSKit is poorly documented. Expect to iterate. The goal is: register a filesystem with FSKit, mount it, see SOMETHING in Finder.

- [ ] **Step 1: Research FSKit API**

Read Apple's FSKit headers (`/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/System/Library/Frameworks/FSKit.framework/Headers/`).

Watch WWDC sessions on FSKit (search Apple Developer for "FSKit" sessions).

Document findings in `docs/ntfs-notes.md` — which classes exist, what the mount lifecycle looks like, how block devices are passed in.

- [ ] **Step 2: Create Info.plist for FSKit extension**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.mnfts.fskit</string>
    <key>CFBundleName</key>
    <string>mNFTS</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>NSExtension</key>
    <dict>
        <key>NSExtensionPointIdentifier</key>
        <string>com.apple.fskit.filesystem</string>
    </dict>
</dict>
</plist>
```

Note: The exact plist keys and extension point identifier may differ. Adjust based on FSKit header research.

- [ ] **Step 3: Implement minimal FSKit filesystem that returns a hardcoded directory listing**

Update `MNFTSFileSystem.swift` based on the actual FSKit API discovered in Step 1. The goal is:
- FSKit loads the extension
- A volume appears in `/Volumes/`
- Finder shows a root directory with one hardcoded test file

This step will require iteration. Expect 2-4 hours of trial and error with FSKit APIs.

- [ ] **Step 4: Test manually**

Run the FSKit extension (method depends on FSKit's actual loading mechanism — may require an Xcode project, `systemextensionsctl`, or similar).

Verify: a volume named "NTFS-Test" appears in Finder with a dummy file.

- [ ] **Step 5: Document FSKit findings**

Create `docs/ntfs-notes.md` with:
- Which FSKit class to subclass (resolved from Phase 0 investigation)
- How block devices are passed to the extension
- How to register/deregister the extension
- Any gotchas, bugs, or undocumented behavior
- Whether `FSUnaryFileSystem` or `FSBlockDeviceFileSystem` is correct

- [ ] **Step 6: Commit**

```bash
git add swift/Sources/MNFTSFSKit/ docs/ntfs-notes.md
git commit -m "feat: FSKit proof of concept — mount dummy filesystem visible in Finder"
```

---

### Task 6: Prove Rust-Swift FFI End-to-End Through FSKit

**Files:**
- Modify: `crates/libmnfts/src/ffi/mod.rs`
- Modify: `swift/Sources/MNFTSFSKit/FFIBridge.swift`
- Modify: `swift/Sources/MNFTSFSKit/MNFTSFileSystem.swift`

- [ ] **Step 1: Add FFI function that returns a hardcoded directory listing**

```rust
// crates/libmnfts/src/ffi/mod.rs (add to existing)

/// A single directory entry returned over FFI.
#[repr(C)]
pub struct MnftsDirEntry {
    pub name: *const std::ffi::c_char,
    pub name_len: u32,
    pub is_directory: bool,
    pub file_size: u64,
}

/// Returns a hardcoded directory listing for testing FFI.
/// Caller must call mnfts_free_dir_entries to free the returned array.
#[no_mangle]
pub extern "C" fn mnfts_test_readdir(count: *mut u32) -> *mut MnftsDirEntry {
    let entries = vec![
        MnftsDirEntry {
            name: "hello.txt\0".as_ptr() as *const std::ffi::c_char,
            name_len: 9,
            is_directory: false,
            file_size: 13,
        },
        MnftsDirEntry {
            name: "Documents\0".as_ptr() as *const std::ffi::c_char,
            name_len: 9,
            is_directory: true,
            file_size: 0,
        },
    ];
    unsafe { *count = entries.len() as u32 };
    let ptr = entries.as_ptr() as *mut MnftsDirEntry;
    std::mem::forget(entries);
    ptr
}
```

- [ ] **Step 2: Call Rust readdir from Swift FSKit module**

Update `FFIBridge.swift` to call `mnfts_test_readdir` and convert results to Swift types.

Update `MNFTSFileSystem.swift` to use the bridge results in the FSKit directory listing callback.

- [ ] **Step 3: Verify in Finder**

Mount the dummy filesystem. Finder should show `hello.txt` and `Documents/` — data sourced from Rust via FFI.

- [ ] **Step 4: Commit**

```bash
git add crates/libmnfts/src/ffi/ swift/Sources/MNFTSFSKit/
git commit -m "feat: prove Rust → Swift FFI through FSKit — Finder shows Rust-generated dir listing"
```

---

## Chunk 2: Block I/O and NTFS Volume Mounting

This chunk implements reading from real NTFS disk images/devices and mounting actual NTFS volumes.

### Task 7: Create Test NTFS Disk Images

**Files:**
- Create: `tests/images/README.md`
- Create: `tests/images/create-images.sh`

- [ ] **Step 1: Create image creation script**

```bash
#!/usr/bin/env bash
# tests/images/create-images.sh
# Creates test NTFS disk images using a Windows VM or mkntfs (if available).
# These images are checked into the repo for reproducible testing.

set -euo pipefail

IMAGES_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Creating basic NTFS image ==="
# Option A: If mkntfs (from ntfs-3g) is available:
# dd if=/dev/zero of="$IMAGES_DIR/basic.img" bs=1M count=16
# mkntfs -F -L "Basic" "$IMAGES_DIR/basic.img"
#
# Option B: If on macOS without mkntfs, create on a Windows VM:
# 1. Create a 16MB VHD
# 2. Format as NTFS with label "Basic"
# 3. Add test files: hello.txt ("Hello, NTFS!"), Documents/readme.txt
# 4. Safely eject
# 5. Copy the raw disk image here

echo "=== Creating dirty journal image ==="
# 1. Create a clean NTFS image
# 2. Mount on Windows
# 3. Start writing a file
# 4. Kill the VM without safe eject
# 5. Copy the image (will have dirty $LogFile)

echo "See README.md for manual creation instructions."
```

- [ ] **Step 2: Create README for test images**

```markdown
# Test NTFS Disk Images

These images are used by integration tests. They must be created manually
because NTFS formatting requires Windows or ntfs-3g tools.

## Required images for v0.1

| Image | Size | Contents | How to create |
|---|---|---|---|
| `basic.img` | 16MB | hello.txt, Documents/readme.txt, unicode/日本語.txt | Format NTFS on Windows, add files |
| `empty.img` | 8MB | Freshly formatted, no files | Format NTFS on Windows |
| `dirty.img` | 16MB | Same as basic but with dirty journal | Yank VM mid-write |
| `deep-paths.img` | 16MB | 30-level nested directories | Create nested dirs on Windows |

## Creation steps (Windows VM)

1. Open Disk Management
2. Create new VHD (fixed size, specified MB)
3. Initialize as MBR
4. Create simple volume, format NTFS
5. Add required files
6. For dirty image: don't safely eject
7. Copy .vhd, extract raw partition with: `dd if=disk.vhd of=image.img bs=512 skip=<partition_offset>`
```

- [ ] **Step 3: Create at least the `basic.img` test image**

This is a manual step. Create a 16MB NTFS image with:
- `hello.txt` containing "Hello, NTFS!\n"
- `Documents/readme.txt` containing "Test document\n"
- A file with Unicode name

If `mkntfs` is available (via Homebrew `ntfs-3g-mac` or similar):
```bash
dd if=/dev/zero of=tests/images/basic.img bs=1M count=16
mkntfs -F -L "Basic" tests/images/basic.img
```

Then mount and add files on a Windows VM, or use `ntfscreate` tools if available.

- [ ] **Step 4: Commit**

```bash
git add tests/images/
git commit -m "test: add NTFS disk image creation scripts and README"
```

---

### Task 8: Implement Block I/O Layer

**Files:**
- Create: `crates/libmnfts/src/io/mod.rs`
- Create: `crates/libmnfts/src/io/block_device.rs`
- Create: `crates/libmnfts/src/io/cache.rs`
- Modify: `crates/libmnfts/src/lib.rs`
- Create: `crates/libmnfts/tests/io_tests.rs`

- [ ] **Step 1: Write failing test for block device reader**

```rust
// crates/libmnfts/tests/io_tests.rs
use libmnfts::io::BlockReader;
use std::io::Cursor;

#[test]
fn test_block_reader_reads_at_offset() {
    let data = vec![0u8; 4096];
    let mut modified = data.clone();
    modified[512] = 0xAB;
    modified[513] = 0xCD;

    let reader = BlockReader::from_reader(Cursor::new(modified), 512);
    let block = reader.read_block(1).unwrap();
    assert_eq!(block[0], 0xAB);
    assert_eq!(block[1], 0xCD);
    assert_eq!(block.len(), 512);
}

#[test]
fn test_block_reader_out_of_bounds() {
    let data = vec![0u8; 1024];
    let reader = BlockReader::from_reader(Cursor::new(data), 512);
    // Only 2 blocks exist (0 and 1)
    assert!(reader.read_block(2).is_err());
}
```

- [ ] **Step 2: Run tests — verify they fail**

Run: `cargo test --test io_tests`
Expected: FAIL — `BlockReader` doesn't exist yet.

- [ ] **Step 3: Implement block I/O module**

```rust
// crates/libmnfts/src/io/mod.rs
mod block_device;
mod cache;

pub use block_device::BlockReader;
pub use cache::BlockCache;
```

```rust
// crates/libmnfts/src/io/block_device.rs
use crate::error::Result;
use std::io::{Read, Seek, SeekFrom};

/// Reads fixed-size blocks from an underlying reader.
/// The reader can be a file, disk image, or FSKit-provided handle.
pub struct BlockReader<R: Read + Seek> {
    reader: std::cell::RefCell<R>,
    block_size: u32,
    total_size: u64,
}

impl<R: Read + Seek> BlockReader<R> {
    pub fn from_reader(mut reader: R, block_size: u32) -> Self {
        let total_size = reader.seek(SeekFrom::End(0)).unwrap_or(0);
        reader.seek(SeekFrom::Start(0)).ok();
        Self {
            reader: std::cell::RefCell::new(reader),
            block_size,
            total_size,
        }
    }

    pub fn block_size(&self) -> u32 {
        self.block_size
    }

    pub fn total_blocks(&self) -> u64 {
        self.total_size / self.block_size as u64
    }

    pub fn read_block(&self, block_num: u64) -> Result<Vec<u8>> {
        let offset = block_num * self.block_size as u64;
        if offset + self.block_size as u64 > self.total_size {
            return Err(crate::error::MnftsError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Block {} out of range (total: {})", block_num, self.total_blocks()),
            )));
        }

        let mut reader = self.reader.borrow_mut();
        reader.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0u8; self.block_size as usize];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_bytes(&self, offset: u64, length: usize) -> Result<Vec<u8>> {
        if offset + length as u64 > self.total_size {
            return Err(crate::error::MnftsError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!("Read at offset {} length {} exceeds volume size {}", offset, length, self.total_size),
            )));
        }

        let mut reader = self.reader.borrow_mut();
        reader.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0u8; length];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }
}
```

- [ ] **Step 4: Run tests — verify they pass**

Run: `cargo test --test io_tests`
Expected: PASS

- [ ] **Step 5: Write failing test for LRU block cache**

```rust
// Add to crates/libmnfts/tests/io_tests.rs

use libmnfts::io::BlockCache;

#[test]
fn test_cache_stores_and_retrieves() {
    let mut cache = BlockCache::new(4); // 4 blocks max
    cache.put(0, vec![0xAA; 512]);
    cache.put(1, vec![0xBB; 512]);

    assert_eq!(cache.get(0).unwrap()[0], 0xAA);
    assert_eq!(cache.get(1).unwrap()[0], 0xBB);
    assert!(cache.get(2).is_none());
}

#[test]
fn test_cache_evicts_lru() {
    let mut cache = BlockCache::new(2); // Only 2 blocks
    cache.put(0, vec![0xAA; 512]);
    cache.put(1, vec![0xBB; 512]);
    // Access block 0 to make it recently used
    cache.get(0);
    // Insert block 2 — should evict block 1 (LRU)
    cache.put(2, vec![0xCC; 512]);

    assert!(cache.get(0).is_some()); // Still present
    assert!(cache.get(1).is_none()); // Evicted
    assert!(cache.get(2).is_some()); // New entry
}
```

- [ ] **Step 6: Run tests — verify they fail**

Run: `cargo test --test io_tests`
Expected: FAIL — `BlockCache` doesn't exist yet.

- [ ] **Step 7: Implement LRU block cache**

```rust
// crates/libmnfts/src/io/cache.rs
use std::collections::{HashMap, VecDeque};

/// Simple LRU cache for block data.
pub struct BlockCache {
    capacity: usize,
    map: HashMap<u64, Vec<u8>>,
    order: VecDeque<u64>,
}

impl BlockCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    pub fn get(&mut self, block_num: u64) -> Option<&Vec<u8>> {
        if self.map.contains_key(&block_num) {
            // Move to back (most recently used)
            self.order.retain(|&b| b != block_num);
            self.order.push_back(block_num);
            self.map.get(&block_num)
        } else {
            None
        }
    }

    pub fn put(&mut self, block_num: u64, data: Vec<u8>) {
        if self.map.contains_key(&block_num) {
            self.order.retain(|&b| b != block_num);
        } else if self.map.len() >= self.capacity {
            if let Some(evicted) = self.order.pop_front() {
                self.map.remove(&evicted);
            }
        }
        self.map.insert(block_num, data);
        self.order.push_back(block_num);
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}
```

- [ ] **Step 8: Run all tests — verify they pass**

Run: `cargo test --test io_tests`
Expected: All PASS

- [ ] **Step 9: Update lib.rs**

```rust
// crates/libmnfts/src/lib.rs
pub mod error;
pub mod ffi;
pub mod io;

pub use error::{MnftsError, Result};
```

- [ ] **Step 10: Commit**

```bash
git add crates/libmnfts/src/io/ crates/libmnfts/src/lib.rs crates/libmnfts/tests/io_tests.rs
git commit -m "feat: implement block I/O layer with LRU cache"
```

---

### Task 9: Implement NTFS Volume Mounting (Boot Sector + Health Checks)

**Files:**
- Create: `crates/libmnfts/src/volume/mod.rs`
- Create: `crates/libmnfts/src/volume/boot_sector.rs`
- Create: `crates/libmnfts/src/volume/health.rs`
- Create: `crates/libmnfts/tests/volume_tests.rs`
- Modify: `crates/libmnfts/src/lib.rs`

- [ ] **Step 1: Write failing test for boot sector validation**

```rust
// crates/libmnfts/tests/volume_tests.rs
use libmnfts::volume::NtfsVolume;
use std::io::Cursor;

#[test]
fn test_mount_rejects_non_ntfs_image() {
    // 16KB of zeros — not a valid NTFS volume
    let data = vec![0u8; 16 * 1024];
    let result = NtfsVolume::open(Cursor::new(data));
    assert!(result.is_err());
}

#[test]
fn test_mount_basic_ntfs_image() {
    let image_data = std::fs::read("tests/images/basic.img")
        .expect("basic.img must exist — see tests/images/README.md");
    let volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    assert!(!volume.label().is_empty());
    assert!(volume.is_healthy());
}
```

- [ ] **Step 2: Run tests — verify they fail**

Run: `cargo test --test volume_tests`
Expected: FAIL — `NtfsVolume` doesn't exist.

- [ ] **Step 3: Implement volume module**

```rust
// crates/libmnfts/src/volume/mod.rs
pub mod boot_sector;
pub mod health;

use crate::error::{MnftsError, Result};
use crate::io::BlockReader;
use ntfs::Ntfs;
use std::io::{Read, Seek};

/// An open, validated NTFS volume (read-only).
pub struct NtfsVolume<R: Read + Seek> {
    ntfs: Ntfs,
    reader: BlockReader<R>,
    label: String,
    healthy: bool,
}

impl<R: Read + Seek> NtfsVolume<R> {
    /// Open and validate an NTFS volume from a reader (file, device, image).
    pub fn open(mut reader: R) -> Result<Self> {
        // Use the ntfs crate to parse the volume
        let ntfs = Ntfs::new(&mut reader).map_err(|_| MnftsError::NotNtfs)?;

        // Validate NTFS version
        let (major, minor) = (ntfs.major_version(), ntfs.minor_version());
        if major != 3 || minor > 1 {
            return Err(MnftsError::UnsupportedVersion(major, minor));
        }

        // Run health checks
        let health_result = health::check_volume_health(&ntfs, &mut reader);
        let healthy = health_result.is_ok();

        // Get volume label
        let label = Self::read_label(&ntfs, &mut reader);

        let block_size = ntfs.cluster_size() as u32;
        let block_reader = BlockReader::from_reader(reader, block_size);

        Ok(Self {
            ntfs,
            reader: block_reader,
            label,
            healthy,
        })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn is_healthy(&self) -> bool {
        self.healthy
    }

    pub fn ntfs(&self) -> &Ntfs {
        &self.ntfs
    }

    fn read_label(ntfs: &Ntfs, reader: &mut R) -> String {
        // Try to read volume label from $Volume file
        ntfs.volume_name(reader)
            .ok()
            .flatten()
            .map(|name| name.name().to_string())
            .unwrap_or_else(|| "NTFS".to_string())
    }
}
```

- [ ] **Step 4: Implement health checks**

```rust
// crates/libmnfts/src/volume/health.rs
use crate::error::{MnftsError, Result};
use ntfs::Ntfs;
use std::io::{Read, Seek};

/// Flags that indicate volume health issues.
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub dirty_journal: bool,
    pub hibernated: bool,
    pub ntfs_version: (u8, u8),
    pub issues: Vec<String>,
}

impl HealthReport {
    pub fn is_clean(&self) -> bool {
        !self.dirty_journal && !self.hibernated && self.issues.is_empty()
    }
}

/// Check volume health — dirty journal, hibernation, etc.
pub fn check_volume_health<R: Read + Seek>(ntfs: &Ntfs, reader: &mut R) -> Result<HealthReport> {
    let mut report = HealthReport {
        dirty_journal: false,
        hibernated: false,
        ntfs_version: (ntfs.major_version(), ntfs.minor_version()),
        issues: Vec::new(),
    };

    // Check volume flags for dirty/hibernation markers
    // The ntfs crate exposes volume flags via the $Volume system file
    if let Ok(volume_info) = ntfs.volume_info(reader) {
        let flags = volume_info.flags();
        // NTFS volume flag 0x0001 = dirty
        // NTFS volume flag 0x0002 = resize journal
        // NTFS volume flag 0x0004 = upgrade on mount
        // Check dirty flag
        if flags.contains(ntfs::NtfsVolumeFlags::DIRTY) {
            report.dirty_journal = true;
            report.issues.push("Volume journal is dirty".to_string());
        }
    }

    // Check for Windows hibernation (hiberfil.sys in root)
    // If hiberfil.sys exists and is non-empty, Windows may be hibernated
    if let Ok(root_dir) = ntfs.root_directory(reader) {
        if let Ok(Some(_)) = root_dir.find_entry(reader, "hiberfil.sys") {
            report.hibernated = true;
            report.issues.push("Windows hibernation file detected".to_string());
        }
    }

    Ok(report)
}
```

Note: The exact `ntfs` crate API for volume flags and directory entry lookup may differ from what's shown. Adjust to match the actual crate API.

- [ ] **Step 5: Update lib.rs**

```rust
// crates/libmnfts/src/lib.rs
pub mod error;
pub mod ffi;
pub mod io;
pub mod volume;

pub use error::{MnftsError, Result};
```

- [ ] **Step 6: Run tests — verify they pass**

Run: `cargo test --test volume_tests`
Expected: PASS (assuming basic.img exists)

- [ ] **Step 7: Commit**

```bash
git add crates/libmnfts/src/volume/ crates/libmnfts/src/lib.rs crates/libmnfts/tests/volume_tests.rs
git commit -m "feat: implement NTFS volume mounting with boot sector validation and health checks"
```

---

## Chunk 3: Filesystem Operations (Read Path)

This chunk implements the core value: reading files and directories from NTFS volumes.

### Task 10: Implement Directory Enumeration

**Files:**
- Create: `crates/libmnfts/src/fs/mod.rs`
- Create: `crates/libmnfts/src/fs/dir.rs`
- Create: `crates/libmnfts/src/fs/metadata.rs`
- Create: `crates/libmnfts/tests/fs_tests.rs`
- Modify: `crates/libmnfts/src/lib.rs`

- [ ] **Step 1: Write failing test for directory listing**

```rust
// crates/libmnfts/tests/fs_tests.rs
use libmnfts::volume::NtfsVolume;
use libmnfts::fs::list_directory;
use std::io::Cursor;

#[test]
fn test_list_root_directory() {
    let image_data = std::fs::read("tests/images/basic.img").unwrap();
    let volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let entries = list_directory(&volume, "/").unwrap();

    // basic.img should have hello.txt and Documents/
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"hello.txt"), "Expected hello.txt in root, got: {:?}", names);
    assert!(names.contains(&"Documents"), "Expected Documents in root, got: {:?}", names);
}

#[test]
fn test_list_subdirectory() {
    let image_data = std::fs::read("tests/images/basic.img").unwrap();
    let volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let entries = list_directory(&volume, "/Documents").unwrap();

    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"readme.txt"), "Expected readme.txt in Documents, got: {:?}", names);
}
```

- [ ] **Step 2: Run tests — verify they fail**

Run: `cargo test --test fs_tests`
Expected: FAIL — `fs` module doesn't exist.

- [ ] **Step 3: Define directory entry types**

```rust
// crates/libmnfts/src/fs/metadata.rs
use std::time::SystemTime;

/// A directory entry with metadata.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_directory: bool,
    pub file_size: u64,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub mft_reference: u64,
}

/// Convert NTFS FILETIME (100ns since 1601-01-01) to SystemTime.
pub fn filetime_to_system_time(filetime: u64) -> Option<SystemTime> {
    // Windows FILETIME epoch is 1601-01-01
    // Unix epoch is 1970-01-01
    // Difference: 11644473600 seconds
    const EPOCH_DIFF_SECS: u64 = 11_644_473_600;
    const HUNDREDS_NANOS_PER_SEC: u64 = 10_000_000;

    let secs_since_1601 = filetime / HUNDREDS_NANOS_PER_SEC;
    if secs_since_1601 < EPOCH_DIFF_SECS {
        return None;
    }
    let unix_secs = secs_since_1601 - EPOCH_DIFF_SECS;
    let nanos = ((filetime % HUNDREDS_NANOS_PER_SEC) * 100) as u32;
    Some(SystemTime::UNIX_EPOCH + std::time::Duration::new(unix_secs, nanos))
}
```

- [ ] **Step 4: Implement directory listing**

```rust
// crates/libmnfts/src/fs/dir.rs
use crate::error::Result;
use crate::fs::metadata::DirEntry;
use crate::volume::NtfsVolume;
use std::io::{Read, Seek};

/// List entries in a directory by path.
pub fn list_directory<R: Read + Seek>(
    volume: &NtfsVolume<R>,
    path: &str,
) -> Result<Vec<DirEntry>> {
    let ntfs = volume.ntfs();
    let mut reader = volume.reader_mut();

    // Navigate to the directory
    let dir = if path == "/" || path.is_empty() {
        ntfs.root_directory(&mut reader)?
    } else {
        // Walk the path components
        let mut current = ntfs.root_directory(&mut reader)?;
        for component in path.trim_start_matches('/').split('/') {
            if component.is_empty() {
                continue;
            }
            let entry = current
                .find_entry(&mut reader, component)?
                .ok_or_else(|| crate::error::MnftsError::Io(
                    std::io::Error::new(std::io::ErrorKind::NotFound, format!("Path not found: {}", component))
                ))?;
            current = entry.to_dir(&ntfs, &mut reader)?;
        }
        current
    };

    // Enumerate entries
    let mut entries = Vec::new();
    let mut iter = dir.entries(&mut reader);

    while let Some(Ok(entry)) = iter.next(&mut reader) {
        let file_name = entry.file_name();
        let name = file_name.name().to_string();

        // Skip NTFS system entries (., .., $MFT, etc.)
        if name.starts_with('$') || name == "." || name == ".." {
            continue;
        }

        // Skip 8.3 short names — only show long names
        if file_name.namespace() == ntfs::NtfsFileNamespace::Dos {
            continue;
        }

        let is_directory = entry.is_directory();
        let file_size = if is_directory { 0 } else { entry.data_size() };

        entries.push(DirEntry {
            name,
            is_directory,
            file_size,
            created: None,  // TODO: extract from $STANDARD_INFORMATION
            modified: None,
            accessed: None,
            mft_reference: entry.mft_reference().file_record_number(),
        });
    }

    Ok(entries)
}
```

Note: The exact `ntfs` crate API for directory iteration, entry access, etc. will need to be adjusted. The crate's API uses `NtfsFile`, `NtfsIndex`, and `NtfsIndexEntry` types — consult the crate docs for the precise method names.

- [ ] **Step 5: Create fs module**

```rust
// crates/libmnfts/src/fs/mod.rs
pub mod dir;
pub mod metadata;

pub use dir::list_directory;
pub use metadata::DirEntry;
```

- [ ] **Step 6: Update lib.rs**

```rust
// crates/libmnfts/src/lib.rs
pub mod error;
pub mod ffi;
pub mod fs;
pub mod io;
pub mod volume;

pub use error::{MnftsError, Result};
```

- [ ] **Step 7: Run tests — verify they pass**

Run: `cargo test --test fs_tests`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add crates/libmnfts/src/fs/ crates/libmnfts/src/lib.rs crates/libmnfts/tests/fs_tests.rs
git commit -m "feat: implement directory enumeration via ntfs crate"
```

---

### Task 11: Implement File Reading

**Files:**
- Create: `crates/libmnfts/src/fs/file.rs`
- Modify: `crates/libmnfts/src/fs/mod.rs`
- Modify: `crates/libmnfts/tests/fs_tests.rs`

- [ ] **Step 1: Write failing test for file reading**

```rust
// Add to crates/libmnfts/tests/fs_tests.rs

use libmnfts::fs::read_file;

#[test]
fn test_read_file_contents() {
    let image_data = std::fs::read("tests/images/basic.img").unwrap();
    let volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let contents = read_file(&volume, "/hello.txt").unwrap();
    assert_eq!(String::from_utf8_lossy(&contents), "Hello, NTFS!\n");
}

#[test]
fn test_read_file_in_subdirectory() {
    let image_data = std::fs::read("tests/images/basic.img").unwrap();
    let volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let contents = read_file(&volume, "/Documents/readme.txt").unwrap();
    assert_eq!(String::from_utf8_lossy(&contents), "Test document\n");
}

#[test]
fn test_read_nonexistent_file() {
    let image_data = std::fs::read("tests/images/basic.img").unwrap();
    let volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let result = read_file(&volume, "/nonexistent.txt");
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run tests — verify they fail**

Run: `cargo test --test fs_tests -- test_read_file`
Expected: FAIL — `read_file` doesn't exist.

- [ ] **Step 3: Implement file reading**

```rust
// crates/libmnfts/src/fs/file.rs
use crate::error::{MnftsError, Result};
use crate::volume::NtfsVolume;
use std::io::{Read, Seek};

/// Read the entire contents of a file by path.
pub fn read_file<R: Read + Seek>(
    volume: &NtfsVolume<R>,
    path: &str,
) -> Result<Vec<u8>> {
    let ntfs = volume.ntfs();
    let mut reader = volume.reader_mut();

    // Navigate to the file
    let components: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let mut current_dir = ntfs.root_directory(&mut reader)?;

    for (i, component) in components.iter().enumerate() {
        if component.is_empty() {
            continue;
        }
        let entry = current_dir
            .find_entry(&mut reader, component)?
            .ok_or_else(|| MnftsError::Io(
                std::io::Error::new(std::io::ErrorKind::NotFound, format!("Not found: {}", component))
            ))?;

        if i == components.len() - 1 {
            // Last component — read the file's data
            let file = entry.to_file(&ntfs, &mut reader)?;
            let data_attr = file
                .data(&mut reader, "")
                .ok_or_else(|| MnftsError::Io(
                    std::io::Error::new(std::io::ErrorKind::NotFound, "No $DATA attribute")
                ))??;

            let data_value = data_attr.to_attribute_value(&mut reader)?;
            let mut buf = Vec::with_capacity(data_value.len() as usize);
            let mut data_reader = data_value.attach(&mut reader);
            data_reader.read_to_end(&mut buf)?;
            return Ok(buf);
        } else {
            current_dir = entry.to_dir(&ntfs, &mut reader)?;
        }
    }

    Err(MnftsError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("File not found: {}", path),
    )))
}

/// Read a portion of a file (for FSKit paged reads).
pub fn read_file_range<R: Read + Seek>(
    volume: &NtfsVolume<R>,
    mft_reference: u64,
    offset: u64,
    length: usize,
) -> Result<Vec<u8>> {
    let ntfs = volume.ntfs();
    let mut reader = volume.reader_mut();

    let file = ntfs.file(&mut reader, mft_reference)?;
    let data_attr = file
        .data(&mut reader, "")
        .ok_or_else(|| MnftsError::Io(
            std::io::Error::new(std::io::ErrorKind::NotFound, "No $DATA attribute")
        ))??;

    let data_value = data_attr.to_attribute_value(&mut reader)?;
    let file_size = data_value.len();

    if offset >= file_size {
        return Ok(Vec::new());
    }

    let actual_length = std::cmp::min(length as u64, file_size - offset) as usize;
    let mut buf = vec![0u8; actual_length];

    let mut data_reader = data_value.attach(&mut reader);
    // Seek to offset within the data stream
    std::io::Seek::seek(&mut data_reader, std::io::SeekFrom::Start(offset))?;
    data_reader.read_exact(&mut buf)?;
    Ok(buf)
}
```

Note: API will need adjustment for actual `ntfs` crate method signatures.

- [ ] **Step 4: Update fs/mod.rs**

```rust
// crates/libmnfts/src/fs/mod.rs
pub mod dir;
pub mod file;
pub mod metadata;

pub use dir::list_directory;
pub use file::{read_file, read_file_range};
pub use metadata::DirEntry;
```

- [ ] **Step 5: Run tests — verify they pass**

Run: `cargo test --test fs_tests`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add crates/libmnfts/src/fs/file.rs crates/libmnfts/src/fs/mod.rs crates/libmnfts/tests/fs_tests.rs
git commit -m "feat: implement file reading from NTFS volumes"
```

---

### Task 12: Implement Attribute Resolution and Timestamp Conversion

**Files:**
- Create: `crates/libmnfts/src/fs/attrs.rs`
- Modify: `crates/libmnfts/src/fs/dir.rs` (add timestamps to DirEntry)
- Modify: `crates/libmnfts/src/fs/mod.rs`
- Modify: `crates/libmnfts/tests/fs_tests.rs`

- [ ] **Step 1: Write failing test for timestamp conversion**

```rust
// Add to crates/libmnfts/tests/fs_tests.rs

use libmnfts::fs::metadata::filetime_to_system_time;
use std::time::{Duration, SystemTime};

#[test]
fn test_filetime_to_system_time_epoch() {
    // 2000-01-01T00:00:00 in Windows FILETIME
    // = 125911584000000000 (100ns ticks since 1601-01-01)
    let ft: u64 = 125_911_584_000_000_000;
    let st = filetime_to_system_time(ft).unwrap();
    let unix_secs = st.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    // 2000-01-01 = 946684800 unix timestamp
    assert_eq!(unix_secs, 946_684_800);
}

#[test]
fn test_dir_entries_have_timestamps() {
    let image_data = std::fs::read("tests/images/basic.img").unwrap();
    let volume = NtfsVolume::open(Cursor::new(image_data)).unwrap();
    let entries = list_directory(&volume, "/").unwrap();

    let hello = entries.iter().find(|e| e.name == "hello.txt").unwrap();
    assert!(hello.modified.is_some(), "Expected modified timestamp");
    assert!(hello.created.is_some(), "Expected created timestamp");
}
```

- [ ] **Step 2: Run tests — verify they fail**

Run: `cargo test --test fs_tests -- test_dir_entries_have_timestamps`
Expected: FAIL — timestamps are None.

- [ ] **Step 3: Implement attribute resolution**

```rust
// crates/libmnfts/src/fs/attrs.rs
use crate::fs::metadata::DirEntry;
use crate::fs::metadata::filetime_to_system_time;
use std::io::{Read, Seek};

/// Enrich a DirEntry with timestamps from $STANDARD_INFORMATION.
pub fn resolve_timestamps<R: Read + Seek>(
    entry: &mut DirEntry,
    file: &ntfs::NtfsFile,
    reader: &mut R,
) {
    // Read $STANDARD_INFORMATION attribute for timestamps
    if let Some(Ok(std_info)) = file.standard_information(reader) {
        entry.created = filetime_to_system_time(std_info.creation_time().nt_timestamp());
        entry.modified = filetime_to_system_time(std_info.modification_time().nt_timestamp());
        entry.accessed = filetime_to_system_time(std_info.access_time().nt_timestamp());
    }
}
```

Note: Adjust for actual `ntfs` crate timestamp API.

- [ ] **Step 4: Update dir.rs to populate timestamps**

Modify `list_directory` in `dir.rs` to call `resolve_timestamps` for each entry.

- [ ] **Step 5: Run tests — verify they pass**

Run: `cargo test --test fs_tests`
Expected: All PASS

- [ ] **Step 6: Commit**

```bash
git add crates/libmnfts/src/fs/
git commit -m "feat: add timestamp resolution from $STANDARD_INFORMATION attributes"
```

---

### Task 13: Wire NTFS Read Path into FSKit Module

**Files:**
- Modify: `crates/libmnfts/src/ffi/mod.rs` (real FFI functions)
- Modify: `crates/libmnfts/src/ffi/types.rs`
- Modify: `swift/Sources/MNFTSFSKit/FFIBridge.swift`
- Modify: `swift/Sources/MNFTSFSKit/MNFTSFileSystem.swift`
- Create: `swift/Sources/MNFTSFSKit/ItemMapping.swift`

- [ ] **Step 1: Add real FFI exports for mount, readdir, read**

Add to `crates/libmnfts/src/ffi/mod.rs`:
- `mnfts_open_volume(fd: i32) -> *mut MnftsVolume` — open NTFS volume from file descriptor
- `mnfts_close_volume(vol: *mut MnftsVolume)` — close and free
- `mnfts_readdir(vol: *mut MnftsVolume, mft_ref: u64, ...) -> MnftsResult` — list directory
- `mnfts_read_file(vol: *mut MnftsVolume, mft_ref: u64, offset: u64, buf: *mut u8, len: u32) -> MnftsResult` — read file data
- `mnfts_stat(vol: *mut MnftsVolume, mft_ref: u64, ...) -> MnftsResult` — get file metadata

Each function validates pointers, catches panics via `std::panic::catch_unwind`, and returns `MnftsResult`.

- [ ] **Step 2: Update Swift FFI bridge**

Update `FFIBridge.swift` to wrap each C function in a Swift-friendly API.

- [ ] **Step 3: Create FSItemID ↔ MFT reference mapping**

```swift
// swift/Sources/MNFTSFSKit/ItemMapping.swift
import FSKit

/// Maps between FSKit's FSItemID and NTFS MFT references.
/// Uses the full 64-bit MFT reference (48-bit record + 16-bit sequence).
enum ItemMapping {
    static func itemID(from mftReference: UInt64) -> FSItemID {
        // FSItemID from raw bytes
        var ref = mftReference
        let data = Data(bytes: &ref, count: 8)
        return FSItemID(data: data)
    }

    static func mftReference(from itemID: FSItemID) -> UInt64 {
        let data = itemID.data
        return data.withUnsafeBytes { $0.load(as: UInt64.self) }
    }
}
```

- [ ] **Step 4: Implement FSKit callbacks using real NTFS data**

Update `MNFTSFileSystem.swift` to implement:
- `lookup` — resolve filename to FSItemID via `mnfts_stat`
- `readDirectory` — enumerate via `mnfts_readdir`
- `read` — read file data via `mnfts_read_file`
- `getattr` — return file metadata via `mnfts_stat`

- [ ] **Step 5: Build and test end-to-end**

Run: `make build`
Mount a real NTFS USB drive (or loopback mount the test image).
Verify in Finder: files visible, can open text files, can copy to Mac.

- [ ] **Step 6: Commit**

```bash
git add crates/libmnfts/src/ffi/ swift/Sources/MNFTSFSKit/
git commit -m "feat: wire NTFS read path into FSKit — real files visible in Finder"
```

---

## Chunk 4: CLI, Diagnostics, and Watchdog

### Task 14: Implement `mnfts mount` and `mnfts unmount` Commands

**Files:**
- Create: `swift/Sources/MNFTSCLI/Commands/MountCommand.swift`
- Create: `swift/Sources/MNFTSCLI/Commands/UnmountCommand.swift`
- Modify: `swift/Sources/MNFTSCLI/main.swift`

- [ ] **Step 1: Implement mount command**

```swift
// swift/Sources/MNFTSCLI/Commands/MountCommand.swift
import ArgumentParser
import Foundation

struct MountCommand: AsyncParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "mount",
        abstract: "Mount an NTFS volume"
    )

    @Argument(help: "Device path (e.g., /dev/disk4s1)")
    var device: String

    @Argument(help: "Mount point (default: /Volumes/<label>)")
    var mountPoint: String?

    @Option(name: .long, help: "Log level: error, warn, info, debug, trace")
    var logLevel: String = "warn"

    @Flag(name: .long, help: "Output JSON (for scripting)")
    var json: Bool = false

    func run() async throws {
        // 1. Validate device exists
        guard FileManager.default.fileExists(atPath: device) else {
            if json {
                print(#"{"error": "Device not found: \#(device)"}"#)
            } else {
                print("Error: Device not found: \(device)")
            }
            throw ExitCode.failure
        }

        // 2. Trigger FSKit mount via management APIs
        // (Exact FSKit API to be determined in Phase 0)
        // FSFileSystemManager.shared.mount(...)

        // 3. Report success
        let mp = mountPoint ?? "/Volumes/NTFS"
        if json {
            print(#"{"status": "mounted", "device": "\#(device)", "mountPoint": "\#(mp)"}"#)
        } else {
            print("Mounted \(device) at \(mp) (read-only)")
        }
    }
}
```

- [ ] **Step 2: Implement unmount command**

```swift
// swift/Sources/MNFTSCLI/Commands/UnmountCommand.swift
import ArgumentParser
import Foundation

struct UnmountCommand: AsyncParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "unmount",
        abstract: "Unmount an NTFS volume"
    )

    @Argument(help: "Mount point or device path")
    var target: String

    @Flag(name: .long, help: "Output JSON")
    var json: Bool = false

    func run() async throws {
        // Trigger FSKit unmount
        // FSFileSystemManager.shared.unmount(...)

        if json {
            print(#"{"status": "unmounted", "target": "\#(target)"}"#)
        } else {
            print("Unmounted \(target)")
        }
    }
}
```

- [ ] **Step 3: Wire commands into CLI main**

```swift
// swift/Sources/MNFTSCLI/main.swift
import ArgumentParser

@main
struct MNFTS: AsyncParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "mnfts",
        abstract: "Mount and manage NTFS volumes on macOS",
        version: "0.1.0",
        subcommands: [
            MountCommand.self,
            UnmountCommand.self,
            StatusCommand.self,
            InspectCommand.self,
            DoctorCommand.self,
        ]
    )
}
```

- [ ] **Step 4: Build and test**

Run: `make build`
Run: `swift run mnfts mount --help`
Expected: Shows mount command help.

- [ ] **Step 5: Commit**

```bash
git add swift/Sources/MNFTSCLI/
git commit -m "feat: implement mnfts mount and unmount CLI commands"
```

---

### Task 15: Implement `mnfts status`, `mnfts inspect`, `mnfts doctor`

**Files:**
- Create: `swift/Sources/MNFTSCLI/Commands/StatusCommand.swift`
- Create: `swift/Sources/MNFTSCLI/Commands/InspectCommand.swift`
- Create: `swift/Sources/MNFTSCLI/Commands/DoctorCommand.swift`
- Create: `crates/libmnfts/src/inspect.rs`
- Create: `crates/libmnfts/src/doctor.rs`
- Modify: `crates/libmnfts/src/ffi/mod.rs`

- [ ] **Step 1: Implement Rust inspect module**

```rust
// crates/libmnfts/src/inspect.rs
use crate::error::Result;
use std::io::{Read, Seek};

/// Raw metadata dump for developers.
#[derive(Debug)]
pub struct VolumeInspection {
    pub ntfs_version: String,
    pub cluster_size: u32,
    pub sector_size: u16,
    pub total_sectors: u64,
    pub mft_record_count: u64,
    pub volume_label: String,
    pub volume_serial: u64,
    pub dirty_flag: bool,
    pub journal_state: String,
}

pub fn inspect_volume<R: Read + Seek>(reader: &mut R) -> Result<VolumeInspection> {
    let ntfs = ntfs::Ntfs::new(reader)?;

    Ok(VolumeInspection {
        ntfs_version: format!("{}.{}", ntfs.major_version(), ntfs.minor_version()),
        cluster_size: ntfs.cluster_size() as u32,
        sector_size: ntfs.sector_size(),
        total_sectors: ntfs.size() / ntfs.sector_size() as u64,
        mft_record_count: 0, // TODO: count MFT records
        volume_label: ntfs.volume_name(reader)
            .ok().flatten()
            .map(|n| n.name().to_string())
            .unwrap_or_default(),
        volume_serial: ntfs.serial_number(),
        dirty_flag: false, // TODO: check volume flags
        journal_state: "clean".to_string(), // TODO: check $LogFile
    })
}
```

- [ ] **Step 2: Implement Rust doctor module**

```rust
// crates/libmnfts/src/doctor.rs
use crate::error::Result;
use crate::volume::health::{check_volume_health, HealthReport};
use std::io::{Read, Seek};

/// Human-readable health report.
#[derive(Debug)]
pub struct DoctorReport {
    pub healthy: bool,
    pub messages: Vec<String>,
}

pub fn check_volume<R: Read + Seek>(reader: &mut R) -> Result<DoctorReport> {
    let ntfs = ntfs::Ntfs::new(reader)?;
    let health = check_volume_health(&ntfs, reader)?;

    let mut messages = Vec::new();

    if health.is_clean() {
        messages.push("Volume is clean and healthy.".to_string());
    }

    if health.dirty_journal {
        messages.push("WARNING: Volume journal is dirty. Run chkdsk on Windows before writing.".to_string());
    }

    if health.hibernated {
        messages.push("WARNING: Windows hibernation detected. Do not write — boot Windows and shut down properly.".to_string());
    }

    messages.push(format!("NTFS version: {}.{}", health.ntfs_version.0, health.ntfs_version.1));

    for issue in &health.issues {
        messages.push(format!("Issue: {}", issue));
    }

    Ok(DoctorReport {
        healthy: health.is_clean(),
        messages,
    })
}
```

- [ ] **Step 3: Add FFI exports for inspect and doctor**

Add to `crates/libmnfts/src/ffi/mod.rs`:
- `mnfts_inspect(fd: i32, ...) -> MnftsResult`
- `mnfts_doctor(fd: i32, ...) -> MnftsResult`

- [ ] **Step 4: Implement Swift CLI commands**

Implement `StatusCommand.swift` (lists mounted volumes), `InspectCommand.swift` (calls `mnfts_inspect` FFI, prints raw metadata or JSON), `DoctorCommand.swift` (calls `mnfts_doctor` FFI, prints health report).

- [ ] **Step 5: Build and test**

Run: `make build`
Run: `swift run mnfts inspect /path/to/basic.img`
Expected: Prints volume metadata.

Run: `swift run mnfts doctor /path/to/basic.img`
Expected: Prints "Volume is clean and healthy."

- [ ] **Step 6: Commit**

```bash
git add crates/libmnfts/src/inspect.rs crates/libmnfts/src/doctor.rs crates/libmnfts/src/ffi/ swift/Sources/MNFTSCLI/
git commit -m "feat: implement mnfts status, inspect, and doctor commands"
```

---

### Task 16: Implement Crash Watchdog

**Files:**
- Create: `swift/Sources/MNFTSWatchdog/main.swift`
- Create: `swift/Sources/MNFTSWatchdog/ProcessMonitor.swift`
- Create: `swift/Sources/MNFTSWatchdog/CrashReporter.swift`
- Create: `swift/Sources/MNFTSWatchdog/com.mnfts.watchdog.plist`

- [ ] **Step 1: Implement process monitor using kqueue**

```swift
// swift/Sources/MNFTSWatchdog/ProcessMonitor.swift
import Foundation

/// Monitors a process by PID using kqueue/EVFILT_PROC.
/// Calls the handler when the process exits.
final class ProcessMonitor {
    private let pid: pid_t
    private let onExit: (pid_t) -> Void
    private var source: DispatchSourceProcess?

    init(pid: pid_t, onExit: @escaping (pid_t) -> Void) {
        self.pid = pid
        self.onExit = onExit
    }

    func start() {
        let source = DispatchSource.makeProcessSource(
            identifier: pid,
            eventMask: .exit,
            queue: .main
        )
        source.setEventHandler { [weak self] in
            guard let self else { return }
            self.onExit(self.pid)
        }
        source.resume()
        self.source = source
    }

    func stop() {
        source?.cancel()
        source = nil
    }
}
```

- [ ] **Step 2: Implement crash reporter**

```swift
// swift/Sources/MNFTSWatchdog/CrashReporter.swift
import Foundation

enum CrashReporter {
    static let logDirectory = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Library/Logs/mNFTS")

    static func writeCrashReport(pid: pid_t, timestamp: Date) {
        try? FileManager.default.createDirectory(
            at: logDirectory, withIntermediateDirectories: true
        )

        let formatter = ISO8601DateFormatter()
        let dateStr = formatter.string(from: timestamp)
        let filename = "crash-\(dateStr).log"
        let fileURL = logDirectory.appendingPathComponent(filename)

        let report = """
        mNFTS Crash Report
        Timestamp: \(dateStr)
        Extension PID: \(pid)
        Status: FSKit extension process exited unexpectedly.
        Action: Volume was force-unmounted for safety.
        """

        try? report.write(to: fileURL, atomically: true, encoding: .utf8)
    }
}
```

- [ ] **Step 3: Implement watchdog main**

```swift
// swift/Sources/MNFTSWatchdog/main.swift
import Foundation

// The watchdog is launched by launchd and monitors the FSKit extension process.
// On crash: force-unmount, write crash report.

// Read the extension PID from launch argument or discovery
guard CommandLine.arguments.count > 1,
      let pid = pid_t(CommandLine.arguments[1]) else {
    print("Usage: mnfts-watchdog <extension-pid>")
    exit(1)
}

print("mNFTS Watchdog: monitoring PID \(pid)")

let monitor = ProcessMonitor(pid: pid) { crashedPid in
    print("mNFTS Watchdog: extension (PID \(crashedPid)) crashed!")

    // Force unmount all mNFTS volumes
    // (FSKit API or diskutil unmount force)
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/usr/sbin/diskutil")
    task.arguments = ["unmount", "force", "all"]  // TODO: target only mNFTS volumes
    try? task.run()

    // Write crash report
    CrashReporter.writeCrashReport(pid: crashedPid, timestamp: Date())

    print("mNFTS Watchdog: volumes force-unmounted, crash report written.")
    exit(0)
}

monitor.start()

// Keep the process running
RunLoop.main.run()
```

- [ ] **Step 4: Create launchd plist**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mnfts.watchdog</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/mnfts-watchdog</string>
    </array>
    <key>KeepAlive</key>
    <false/>
    <key>RunAtLoad</key>
    <false/>
</dict>
</plist>
```

- [ ] **Step 5: Add watchdog target to Package.swift**

Add an executable target for `MNFTSWatchdog` in `swift/Package.swift`.

- [ ] **Step 6: Build and test**

Run: `make build`
Test: Start a dummy process, launch watchdog with its PID, kill the process, verify crash report appears in `~/Library/Logs/mNFTS/`.

- [ ] **Step 7: Commit**

```bash
git add swift/Sources/MNFTSWatchdog/ swift/Package.swift
git commit -m "feat: implement crash watchdog with kqueue process monitoring"
```

---

## Chunk 5: Testing, Hardening, and Release

### Task 17: Comprehensive Test Suite

**Files:**
- Create: `crates/libmnfts/tests/mount_and_read.rs`
- Create: `crates/libmnfts/tests/health_checks.rs`
- Create: `crates/libmnfts/tests/unicode_tests.rs`
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Write integration tests against golden images**

```rust
// crates/libmnfts/tests/mount_and_read.rs
use libmnfts::volume::NtfsVolume;
use libmnfts::fs::{list_directory, read_file};
use std::io::Cursor;

fn open_test_image(name: &str) -> NtfsVolume<Cursor<Vec<u8>>> {
    let path = format!("tests/images/{}", name);
    let data = std::fs::read(&path)
        .unwrap_or_else(|_| panic!("Test image not found: {}. See tests/images/README.md", path));
    NtfsVolume::open(Cursor::new(data)).unwrap()
}

#[test]
fn test_mount_and_list_root() {
    let vol = open_test_image("basic.img");
    let entries = list_directory(&vol, "/").unwrap();
    assert!(!entries.is_empty(), "Root directory should not be empty");
}

#[test]
fn test_read_known_file() {
    let vol = open_test_image("basic.img");
    let data = read_file(&vol, "/hello.txt").unwrap();
    assert!(!data.is_empty(), "hello.txt should have content");
}

#[test]
fn test_deep_directory_traversal() {
    let vol = open_test_image("deep-paths.img");
    // Try to list a deeply nested directory
    let mut path = String::from("/");
    for i in 0..10 {
        path.push_str(&format!("level{}/", i));
    }
    let result = list_directory(&vol, &path);
    assert!(result.is_ok(), "Should handle deep paths: {:?}", result.err());
}
```

- [ ] **Step 2: Write health check tests**

```rust
// crates/libmnfts/tests/health_checks.rs
use libmnfts::volume::NtfsVolume;
use std::io::Cursor;

#[test]
fn test_dirty_volume_reports_unhealthy() {
    let data = std::fs::read("tests/images/dirty.img")
        .expect("dirty.img needed — see tests/images/README.md");
    let vol = NtfsVolume::open(Cursor::new(data)).unwrap();
    assert!(!vol.is_healthy(), "Dirty volume should report unhealthy");
}

#[test]
fn test_clean_volume_reports_healthy() {
    let data = std::fs::read("tests/images/basic.img").unwrap();
    let vol = NtfsVolume::open(Cursor::new(data)).unwrap();
    assert!(vol.is_healthy(), "Clean volume should report healthy");
}

#[test]
fn test_non_ntfs_image_rejected() {
    let data = vec![0u8; 16 * 1024]; // All zeros
    assert!(NtfsVolume::open(Cursor::new(data)).is_err());
}
```

- [ ] **Step 3: Write Unicode filename tests**

```rust
// crates/libmnfts/tests/unicode_tests.rs
use libmnfts::volume::NtfsVolume;
use libmnfts::fs::list_directory;
use std::io::Cursor;

#[test]
fn test_unicode_filenames() {
    let data = std::fs::read("tests/images/basic.img").unwrap();
    let vol = NtfsVolume::open(Cursor::new(data)).unwrap();
    let entries = list_directory(&vol, "/").unwrap();

    // Check that Unicode filenames are properly decoded
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    // basic.img should contain a file with Japanese characters
    let has_unicode = names.iter().any(|n| n.chars().any(|c| !c.is_ascii()));
    assert!(has_unicode, "Expected at least one Unicode filename, got: {:?}", names);
}
```

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: All pass.

- [ ] **Step 5: Commit**

```bash
git add crates/libmnfts/tests/
git commit -m "test: add integration tests for mount, read, health checks, and Unicode"
```

---

### Task 18: Set Up CI Pipeline

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create CI workflow**

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  rust-checks:
    name: Rust Build & Test
    runs-on: macos-15
    steps:
      - uses: actions/checkout@v4
        with:
          lfs: true

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Check formatting
        run: cargo fmt -- --check

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Build
        run: cargo build --release

      - name: Test
        run: cargo test

  swift-build:
    name: Swift Build
    runs-on: macos-15
    needs: rust-checks
    steps:
      - uses: actions/checkout@v4
        with:
          lfs: true

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Build
        run: make build

  fuzz:
    name: Fuzz (10 min)
    runs-on: macos-15
    needs: rust-checks
    steps:
      - uses: actions/checkout@v4
        with:
          lfs: true

      - name: Install Rust nightly
        uses: dtolnay/rust-toolchain@nightly

      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz

      - name: Fuzz MFT parser (10 min)
        run: cargo +nightly fuzz run mft_parser -- -max_total_time=600
        continue-on-error: true
```

- [ ] **Step 2: Commit**

```bash
git add .github/
git commit -m "ci: add GitHub Actions workflow for build, test, lint, and fuzz"
```

---

### Task 19: Create Project Documentation

**Files:**
- Create: `README.md`
- Create: `LICENSE`
- Create: `CONTRIBUTING.md`
- Create: `CLAUDE.md`
- Create: `.github/SECURITY.md`
- Create: `.github/ISSUE_TEMPLATE/bug_report.md`

- [ ] **Step 1: Create MIT LICENSE**

```
MIT License

Copyright (c) 2026 mNFTS Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 2: Create README.md**

Cover: what mNFTS is, current status (v0.1 read-only), installation (`brew install mnfts`), quick start (`mnfts mount /dev/disk4s1`), commands reference, known limitations, contributing link, license.

- [ ] **Step 3: Create CONTRIBUTING.md**

Cover: build prerequisites (Rust, Xcode, macOS 15.4+), build instructions (`make build`), test instructions (`make test`), contribution tiers (from spec §13), PR requirements, code style (`cargo fmt`, `cargo clippy`, `swiftformat`).

- [ ] **Step 4: Create CLAUDE.md**

```markdown
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

- `make build` — Build Rust core + Swift CLI (requires Rust toolchain + Xcode)
- `make test` — Run all tests
- `make lint` — Check formatting and clippy warnings
- `cargo test --test <name>` — Run a specific Rust test file
- `cargo test <test_fn>` — Run a single test by name

## Architecture

mNFTS is a read-only NTFS driver for macOS 15.4+ using FSKit.

- **crates/libmnfts/** — Rust core: NTFS parsing (via `ntfs` crate), block I/O, health checks, FFI exports
- **swift/Sources/MNFTSFSKit/** — Swift FSKit extension: thin shell calling libmnfts via C-ABI
- **swift/Sources/MNFTSCLI/** — Swift CLI: `mnfts mount/unmount/status/inspect/doctor`
- **swift/Sources/MNFTSWatchdog/** — Swift launchd agent: monitors FSKit extension, force-unmounts on crash

Data flow: Finder → VFS → fskitd → MNFTSFSKit (Swift) → libmnfts (Rust) → ntfs crate → block device

## Key Constraints

- Zero `unsafe` in Rust NTFS/journal/safety modules. `unsafe` only in `ffi/` and `io/` modules.
- v0.1 is read-only — no write code paths exist.
- Block device access via FSKit's provided resource handle, not direct `/dev/rdisk`.
- All tests require golden NTFS images in `tests/images/` — see `tests/images/README.md`.
```

- [ ] **Step 5: Create SECURITY.md and bug report template**

- [ ] **Step 6: Commit**

```bash
git add README.md LICENSE CONTRIBUTING.md CLAUDE.md .github/
git commit -m "docs: add README, LICENSE, CONTRIBUTING, CLAUDE.md, and GitHub templates"
```

---

### Task 20: Create Homebrew Formula

**Files:**
- Create: `Formula/mnfts.rb`

- [ ] **Step 1: Create Homebrew formula**

```ruby
# Formula/mnfts.rb
class Mnfts < Formula
  desc "Free, open-source NTFS driver for macOS (read-only)"
  homepage "https://github.com/youruser/mnfts"
  url "https://github.com/youruser/mnfts/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER"
  license "MIT"

  depends_on :macos
  depends_on xcode: ["16.0", :build]

  def install
    system "cargo", "build", "--release", "--manifest-path", "Cargo.toml"
    cd "swift" do
      system "swift", "build", "-c", "release"
    end
    bin.install "swift/.build/release/mnfts"
    bin.install "swift/.build/release/mnfts-watchdog"
  end

  def post_install
    # Register FSKit extension
    # (Exact mechanism TBD based on Phase 0 FSKit research)
    ohai "Run 'mnfts mount /dev/diskNsN' to mount an NTFS volume"
  end

  test do
    assert_match "mnfts 0.1.0", shell_output("#{bin}/mnfts version")
  end
end
```

- [ ] **Step 2: Test formula locally**

Run: `brew install --build-from-source Formula/mnfts.rb`
Expected: Installs, `mnfts version` works.

- [ ] **Step 3: Commit**

```bash
git add Formula/
git commit -m "chore: add Homebrew formula for mnfts"
```

---

### Task 21: End-to-End Validation and v0.1 Tag

- [ ] **Step 1: Run full test suite**

Run: `make test`
Run: `make lint`
Expected: All pass, zero warnings.

- [ ] **Step 2: Manual end-to-end test**

1. Plug in a real NTFS-formatted USB drive
2. `mnfts mount /dev/diskNsN`
3. Verify volume appears in Finder
4. Browse directories
5. Open a text file (Quick Look)
6. Copy a file to ~/Desktop
7. Verify copied file's SHA256 matches original
8. `mnfts unmount /Volumes/<label>`
9. Verify clean unmount

- [ ] **Step 3: Test watchdog**

1. Mount a volume
2. `kill -9` the FSKit extension process
3. Verify watchdog force-unmounts within 2 seconds
4. Verify crash report in `~/Library/Logs/mNFTS/`

- [ ] **Step 4: Test diagnostics**

1. `mnfts inspect /dev/diskNsN` — verify metadata output
2. `mnfts doctor /dev/diskNsN` — verify "healthy" report
3. `mnfts doctor /path/to/dirty.img` — verify dirty warning

- [ ] **Step 5: Tag v0.1.0**

```bash
git tag -a v0.1.0 -m "v0.1.0: Read-only MVP"
```

- [ ] **Step 6: Commit any final fixes**

```bash
git commit -m "chore: v0.1.0 release preparation"
```

---

## Dependencies Between Tasks

```
Task 1 (Rust workspace)
  → Task 2 (cbindgen)
    → Task 3 (Swift package)
      → Task 4 (Makefile)
        → Task 5 (FSKit PoC) ← PHASE 0 EXIT GATE
          → Task 6 (FFI end-to-end)

Task 7 (test images) — independent, do early

Task 8 (Block I/O)
  → Task 9 (Volume mounting)
    → Task 10 (Dir enumeration)
      → Task 11 (File reading)
        → Task 12 (Attributes/timestamps)
          → Task 13 (Wire into FSKit)

Task 14 (CLI mount/unmount) — after Task 5
Task 15 (CLI diagnostics) — after Task 9
Task 16 (Watchdog) — after Task 5

Task 17 (Tests) — after Tasks 10-12
Task 18 (CI) — after Task 17
Task 19 (Docs) — after Task 14
Task 20 (Homebrew) — after Task 19
Task 21 (E2E validation) — after all above
```

## Parallelization Opportunities

These tasks can run in parallel:
- **Task 7** (test images) alongside Tasks 1-6
- **Task 14** (CLI) alongside Tasks 8-12 (core read path)
- **Task 16** (watchdog) alongside Tasks 8-12
- **Task 18** (CI) alongside Task 19 (docs)
- **Task 19** (docs) alongside Task 20 (Homebrew)

---

## Errata: Critical and High Issue Fixes

These corrections address issues found during plan review and **override** the corresponding sections above.

### E1: NtfsVolume Ownership Model (Fixes C2 — compile failure)

The `ntfs` crate's `Ntfs` struct does NOT own the reader — it requires `&mut reader` passed into every operation. Storing `Ntfs` and `BlockReader<R>` (which owns the reader) as separate struct fields creates a borrow conflict.

**Corrected design for Task 9:**

```rust
// crates/libmnfts/src/volume/mod.rs — CORRECTED

use crate::error::{MnftsError, Result};
use ntfs::Ntfs;
use std::io::{Read, Seek};

/// An open, validated NTFS volume (read-only).
/// The reader is stored alongside Ntfs, and both are accessed
/// through methods that borrow &mut self to avoid split-borrow conflicts.
pub struct NtfsVolume<R: Read + Seek> {
    ntfs: Ntfs,
    reader: R,
    label: String,
    healthy: bool,
}

impl<R: Read + Seek> NtfsVolume<R> {
    pub fn open(mut reader: R) -> Result<Self> {
        let ntfs = Ntfs::new(&mut reader).map_err(|_| MnftsError::NotNtfs)?;

        let (major, minor) = (ntfs.major_version(), ntfs.minor_version());
        if major != 3 || minor > 1 {
            return Err(MnftsError::UnsupportedVersion(major, minor));
        }

        let health = crate::volume::health::check_volume_health(&ntfs, &mut reader);
        let healthy = health.map(|h| h.is_clean()).unwrap_or(false);
        let label = Self::read_label(&ntfs, &mut reader);

        Ok(Self { ntfs, reader, label, healthy })
    }

    /// All filesystem operations go through this method, which gives
    /// mutable access to both ntfs and reader without split-borrow issues.
    pub fn with_ntfs<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&Ntfs, &mut R) -> T,
    {
        f(&self.ntfs, &mut self.reader)
    }

    pub fn label(&self) -> &str { &self.label }
    pub fn is_healthy(&self) -> bool { self.healthy }

    fn read_label(ntfs: &Ntfs, reader: &mut R) -> String {
        // Use ntfs crate's actual API — see E5 for API notes
        "NTFS".to_string() // placeholder — implement per ntfs crate docs
    }
}
```

**Impact on Tasks 10, 11, 12:** All functions that take `&NtfsVolume` must instead take `&mut NtfsVolume` and use `volume.with_ntfs(|ntfs, reader| { ... })`. For example:

```rust
// Corrected list_directory signature
pub fn list_directory<R: Read + Seek>(
    volume: &mut NtfsVolume<R>,
    path: &str,
) -> Result<Vec<DirEntry>> {
    volume.with_ntfs(|ntfs, reader| {
        let root = ntfs.root_directory(reader)?;
        // ... enumerate entries using reader ...
        Ok(entries)
    })
}
```

The `BlockReader` with LRU cache (Task 8) is **still built** but serves as a wrapping layer: `NtfsVolume<CachedBlockReader<File>>` where `CachedBlockReader` implements `Read + Seek` and internally caches block reads. This gives caching without split-borrow issues.

---

### E2: FFI Memory Ownership Protocol (Fixes C1 — use-after-free)

**Task 6 and Task 13 must use a callback-based FFI pattern for directory enumeration, not returned pointer arrays.**

```rust
// Corrected FFI readdir pattern — callback-based

/// Callback type: Swift provides this function, Rust calls it once per entry.
pub type MnftsDirEntryCallback = extern "C" fn(
    context: *mut std::ffi::c_void,  // Swift passes self pointer
    name: *const u8,                  // UTF-8 name bytes (valid only during callback)
    name_len: u32,
    is_directory: bool,
    file_size: u64,
    mft_reference: u64,
    created_secs: i64,               // Unix timestamp or -1 if unknown
    modified_secs: i64,
    accessed_secs: i64,
);

/// Enumerate directory entries. Calls `callback` for each entry.
/// Rust owns all memory; Swift copies what it needs inside the callback.
#[no_mangle]
pub extern "C" fn mnfts_readdir(
    vol: *mut MnftsVolumeHandle,
    parent_mft_ref: u64,
    callback: MnftsDirEntryCallback,
    context: *mut std::ffi::c_void,
) -> MnftsResult {
    // ... iterate entries, call callback for each ...
    MnftsResult::Ok
}
```

**Swift side:**
```swift
// Swift callback that FSKit module provides
let callback: @convention(c) (UnsafeMutableRawPointer?, ...) -> Void = { context, namePtr, nameLen, isDir, size, mftRef, created, modified, accessed in
    let entries = Unmanaged<NSMutableArray>.fromOpaque(context!).takeUnretainedValue()
    let name = String(bytes: UnsafeBufferPointer(start: namePtr, count: Int(nameLen)), encoding: .utf8) ?? ""
    // Build FSKit directory entry and append to entries array
}
```

This eliminates all dangling pointer risks — Rust owns the memory, Swift copies during the callback, nothing is freed prematurely.

**For `mnfts_stat` and file metadata:** Use a `repr(C)` output struct that Rust fills in (caller-allocated, not Rust-allocated):

```rust
#[repr(C)]
pub struct MnftsFileInfo {
    pub file_size: u64,
    pub is_directory: bool,
    pub mft_reference: u64,
    pub created_secs: i64,
    pub modified_secs: i64,
    pub accessed_secs: i64,
}

#[no_mangle]
pub extern "C" fn mnfts_stat(
    vol: *mut MnftsVolumeHandle,
    mft_ref: u64,
    out: *mut MnftsFileInfo,  // Caller-allocated
) -> MnftsResult { ... }
```

---

### E3: Block Device Access — Phase 0 Must Resolve Before Tasks 8-9 (Fixes C3)

**Corrected dependency chain:**

```
Task 5 (FSKit PoC) ← MUST resolve block device access model
  → Task 5b (NEW): Document block device access pattern
  → Tasks 8, 9 (Block I/O and Volume mounting) — now depend on Task 5b
```

**New Task 5b: Resolve FSKit Block Device Access Model**

After Task 5 (FSKit PoC), before Task 8:

- [ ] **Step 1:** Determine how FSKit passes the block device to the extension. Options:
  - `FSBlockDeviceResource` with async read methods
  - Raw file descriptor
  - Memory-mapped region
- [ ] **Step 2:** If FSKit provides async reads, create a synchronous `Read + Seek` adapter in Swift that bridges to Rust. This adapter sits in `MNFTSFSKit/BlockDeviceAdapter.swift`.
- [ ] **Step 3:** If FSKit provides a file descriptor, pass it directly to Rust via FFI.
- [ ] **Step 4:** Document the chosen pattern in `docs/ntfs-notes.md`.
- [ ] **Step 5:** Update `BlockReader` (Task 8) to accept whatever abstraction FSKit provides.

**The Rust `BlockReader<R: Read + Seek>` design is still correct** — the variable is what `R` is. It might be `File` (from fd), or `SyncBlockDeviceAdapter` (wrapping FSKit's async API). Task 5b resolves this.

---

### E4: Hibernation Detection Fix (Fixes H2)

Replace the hibernation check in Task 9 `health.rs`:

```rust
// Corrected hibernation detection
// Hibernation concern: Windows Fast Startup (hybrid shutdown) sets dirty flag
// AND leaves a non-zero hiberfil.sys. Check BOTH conditions.

if let Ok(root_dir) = ntfs.root_directory(reader) {
    if let Ok(Some(entry)) = root_dir.find_entry(reader, "hiberfil.sys") {
        let file = entry.to_file(ntfs, reader);
        if let Ok(f) = file {
            // Only flag hibernation if hiberfil.sys is non-zero AND journal is dirty
            let size = f.data_size();
            if size > 0 && report.dirty_journal {
                report.hibernated = true;
                report.issues.push(
                    "Windows hibernation/Fast Startup detected (non-zero hiberfil.sys + dirty journal). \
                     Boot Windows and do a full shutdown (not hibernate/fast startup) before writing."
                    .to_string()
                );
            }
        }
    }
}
```

---

### E5: `ntfs` Crate API Audit Note (Fixes H1)

**Before implementing Tasks 9-12, the implementor MUST read the `ntfs` crate documentation and source.**

The code samples in Tasks 9-12 use approximate API calls. Key corrections:

| Plan uses | Actual `ntfs` crate API (approximate) |
|---|---|
| `ntfs.volume_name(reader)` | Access `$Volume` system file → read `NtfsVolumeName` attribute |
| `ntfs.volume_info(reader)` | Access `$Volume` system file → `NtfsVolumeInformation` |
| `root_dir.find_entry(reader, name)` | `NtfsFile` → `NtfsIndex` over `$I30` → `NtfsIndexEntry` iteration with name comparison |
| `entry.to_dir(ntfs, reader)` | `NtfsIndexEntry` → `NtfsFile` → check if directory |
| `entry.to_file(ntfs, reader)` | `NtfsIndexEntry` → `NtfsFile` (via MFT reference) |
| `file.data(reader, "")` | `NtfsFile` → iterate attributes → find `NtfsAttributeType::Data` with empty name |
| `entry.file_name()` | `NtfsIndexEntry::file_name()` → `NtfsFileName` |
| `entry.is_directory()` | Check `NtfsFileAttributeFlags` from `NtfsFileName` |

**Action:** Insert a "Task 8b: ntfs Crate API Audit" step where the implementor:
1. Reads `ntfs` crate docs at docs.rs/ntfs
2. Runs the crate's own examples against a test image
3. Documents the actual API mapping in `docs/ntfs-notes.md`

---

### E5b: Concurrent Access — Mutex in FFI Layer (NEW-1)

FSKit may dispatch `readDirectory` and `read` calls concurrently from different threads. The `with_ntfs` / `&mut self` pattern serializes all access, which is correct, but the FFI layer must enforce this.

**Task 13 must wrap the volume handle in a Mutex:**

```rust
// crates/libmnfts/src/ffi/mod.rs

use std::sync::Mutex;

/// Opaque FFI handle wrapping the volume in a Mutex for thread safety.
pub struct MnftsVolumeHandle {
    inner: Mutex<NtfsVolume<Box<dyn ReadSeek>>>,
}

// All FFI functions lock the mutex before accessing the volume:
#[no_mangle]
pub extern "C" fn mnfts_readdir(
    vol: *mut MnftsVolumeHandle,
    parent_mft_ref: u64,
    callback: MnftsDirEntryCallback,
    context: *mut std::ffi::c_void,
) -> MnftsResult {
    let handle = unsafe { &*vol };
    let mut volume = match handle.inner.lock() {
        Ok(v) => v,
        Err(_) => return MnftsResult::ErrInternal, // mutex poisoned
    };
    // ... use volume.with_ntfs(...) ...
    MnftsResult::Ok
}
```

This serializes all filesystem operations through a single lock, which is correct for v0.1 (correctness over concurrency). Performance optimization (reader pooling, lock-free reads) can come later.

---

### E6: Watchdog PID Discovery (Fixes H3)

Replace Task 16's PID-from-argument approach:

The watchdog should discover the FSKit extension PID via the mount command. When `mnfts mount` triggers the FSKit extension, it writes the extension's PID to a file:

```
~/.mnfts/mounts/<device-hash>.pid
```

The watchdog reads this file. If the extension restarts (FSKit may restart it), the mount command updates the PID file.

Add to Task 16 Step 3 (`main.swift`):

```swift
// Discover extension PID from PID file
let pidDir = FileManager.default.homeDirectoryForCurrentUser
    .appendingPathComponent(".mnfts/mounts")
// Watch directory for .pid files
// For each .pid file, monitor that PID
```

Also, the watchdog's force-unmount must target only mNFTS volumes (fixes M9):

```swift
// Read mount info from ~/.mnfts/mounts/<hash>.mount (contains device + mount point)
// Force-unmount only those specific mount points
task.arguments = ["unmount", "force", mountPoint]
```

---

### E7: `mnfts emergency-unmount` Command (Fixes H4)

**Add to Task 14:**

```swift
// swift/Sources/MNFTSCLI/Commands/EmergencyUnmountCommand.swift
struct EmergencyUnmountCommand: ParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "emergency-unmount",
        abstract: "Immediately disconnect an NTFS volume (no flush, no journal write)"
    )

    @Argument(help: "Device path or mount point")
    var target: String

    func run() throws {
        // 1. Force-close the block device (bypass normal unmount)
        // 2. Deregister from FSKit
        // 3. No flush, no journal, no cache write-back
        print("Emergency unmount: \(target) disconnected immediately.")
        print("Warning: volume may need chkdsk on next mount.")
    }
}
```

Add `EmergencyUnmountCommand.self` to the CLI's subcommands list.

---

## Chunk 6: Missing v0.1 Features

These tasks cover v0.1 spec requirements that were missing from the original plan.

### Task 22: Corrupt MFT Record Handling (Skip and Continue)

**Files:**
- Modify: `crates/libmnfts/src/fs/dir.rs`
- Create: `crates/libmnfts/tests/corrupt_volume_tests.rs`

Per spec §7.6: "MFT record number mismatch → skip record, log, continue."

- [ ] **Step 1: Write failing test**

```rust
// crates/libmnfts/tests/corrupt_volume_tests.rs
use libmnfts::volume::NtfsVolume;
use libmnfts::fs::list_directory;
use std::io::Cursor;

#[test]
fn test_corrupt_mft_skips_bad_records() {
    let data = std::fs::read("tests/images/corrupt-mft.img")
        .expect("corrupt-mft.img needed");
    let mut vol = NtfsVolume::open(Cursor::new(data)).unwrap(); // Should not fail
    let entries = list_directory(&mut vol, "/").unwrap();
    // Should return some entries (the non-corrupt ones)
    assert!(!entries.is_empty(), "Should skip corrupt records, not fail entirely");
}
```

- [ ] **Step 2: Run test — verify it fails**

Expected: FAIL (panic or error on corrupt MFT record)

- [ ] **Step 3: Add error recovery in directory enumeration**

Wrap the entry iteration in `list_directory` with error handling that logs and skips:

```rust
while let Some(result) = iter.next(reader) {
    match result {
        Ok(entry) => { /* process normally */ },
        Err(e) => {
            tracing::warn!("Skipping corrupt MFT entry: {}", e);
            continue;
        }
    }
}
```

- [ ] **Step 4: Run test — verify it passes**
- [ ] **Step 5: Commit**

```bash
git commit -m "feat: skip corrupt MFT records during directory enumeration"
```

---

### Task 23: Large File Read Support (>4GB)

**Files:**
- Create: `crates/libmnfts/tests/large_file_tests.rs`

Per spec §8: Large files >4GB read support in v0.1.

- [ ] **Step 1: Write test**

```rust
#[test]
fn test_read_large_file_metadata() {
    let data = std::fs::read("tests/images/large-files.img")
        .expect("large-files.img needed");
    let mut vol = NtfsVolume::open(Cursor::new(data)).unwrap();
    let entries = list_directory(&mut vol, "/").unwrap();
    let large = entries.iter().find(|e| e.file_size > 4_294_967_296).unwrap();
    assert!(large.file_size > 4_294_967_296, "Should report correct size for >4GB files");
}

#[test]
fn test_read_large_file_partial() {
    let data = std::fs::read("tests/images/large-files.img").unwrap();
    let mut vol = NtfsVolume::open(Cursor::new(data)).unwrap();
    // Read 4KB from offset 5GB into the file
    let entries = list_directory(&mut vol, "/").unwrap();
    let large = entries.iter().find(|e| e.file_size > 4_294_967_296).unwrap();
    let chunk = read_file_range(&mut vol, large.mft_reference, 5_368_709_120, 4096).unwrap();
    assert_eq!(chunk.len(), 4096);
}
```

- [ ] **Step 2: Verify these pass with existing read implementation**

The `ntfs` crate handles multi-extent data runs natively. If the tests pass without changes, this feature is already covered. If not, debug the data run walker.

- [ ] **Step 3: Commit**

```bash
git commit -m "test: verify large file (>4GB) read support"
```

---

### Task 24: Sparse and Compressed File Detection

**Files:**
- Modify: `crates/libmnfts/src/fs/dir.rs`
- Modify: `crates/libmnfts/src/fs/metadata.rs`
- Create: `crates/libmnfts/tests/special_files_tests.rs`

Per spec §7.6: Sparse files return zeros for unallocated regions. Compressed files return ENOTSUP.

- [ ] **Step 1: Add flags to DirEntry**

```rust
// Add to DirEntry in metadata.rs
pub struct DirEntry {
    // ... existing fields ...
    pub is_sparse: bool,
    pub is_compressed: bool,
    pub is_encrypted: bool,
}
```

- [ ] **Step 2: Detect flags during enumeration**

In `list_directory`, check `NtfsFileAttributeFlags` for sparse, compressed, and encrypted bits.

- [ ] **Step 3: Handle compressed file reads**

In `read_file` / `read_file_range`, if the file is compressed, return `MnftsError::Io` with `ErrorKind::Unsupported` and message "Compressed NTFS files are not supported. File is visible but cannot be read."

- [ ] **Step 4: Write tests**

```rust
#[test]
fn test_compressed_file_detected() {
    let data = std::fs::read("tests/images/compressed.img").unwrap();
    let mut vol = NtfsVolume::open(Cursor::new(data)).unwrap();
    let entries = list_directory(&mut vol, "/").unwrap();
    let compressed = entries.iter().find(|e| e.is_compressed);
    assert!(compressed.is_some(), "Should detect compressed files");
}

#[test]
fn test_compressed_file_read_returns_error() {
    let data = std::fs::read("tests/images/compressed.img").unwrap();
    let mut vol = NtfsVolume::open(Cursor::new(data)).unwrap();
    let entries = list_directory(&mut vol, "/").unwrap();
    let compressed = entries.iter().find(|e| e.is_compressed).unwrap();
    let result = read_file_range(&mut vol, compressed.mft_reference, 0, 100);
    assert!(result.is_err(), "Reading compressed file should return error");
}
```

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: detect sparse/compressed/encrypted files, return ENOTSUP for compressed reads"
```

---

### Task 25: Set Up Fuzz Targets

**Files:**
- Create: `crates/mnfts-fuzz/Cargo.toml`
- Create: `crates/mnfts-fuzz/fuzz/fuzz_targets/mft_parser.rs`

Per spec §11 and CI workflow (Task 18).

- [ ] **Step 1: Initialize fuzz crate**

```bash
cd crates && cargo fuzz init --fuzz-dir mnfts-fuzz
```

Or manually create:

```toml
# crates/mnfts-fuzz/Cargo.toml
[package]
name = "mnfts-fuzz"
version = "0.0.0"
publish = false
edition = "2024"

[dependencies]
libfuzzer-sys = "0.4"
libmnfts = { path = "../libmnfts" }
ntfs = "0.4"
```

- [ ] **Step 2: Create MFT parser fuzz target**

```rust
// crates/mnfts-fuzz/fuzz/fuzz_targets/mft_parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    // Try to open arbitrary bytes as an NTFS volume
    // Should never panic, only return errors gracefully
    let _ = libmnfts::volume::NtfsVolume::open(Cursor::new(data.to_vec()));
});
```

- [ ] **Step 3: Run briefly to verify it works**

Run: `cd crates/mnfts-fuzz && cargo +nightly fuzz run mft_parser -- -max_total_time=30`
Expected: Runs for 30 seconds without crashes.

- [ ] **Step 4: Commit**

```bash
git add crates/mnfts-fuzz/
git commit -m "test: add fuzz targets for MFT parser"
```

---

### Task 26: Configure Git LFS for Test Images

**Files:**
- Create: `.gitattributes`

- [ ] **Step 1: Install and configure Git LFS**

```bash
git lfs install
```

- [ ] **Step 2: Create .gitattributes**

```
# Track NTFS disk images with Git LFS
tests/images/*.img filter=lfs diff=lfs merge=lfs -text
```

- [ ] **Step 3: Commit**

```bash
git add .gitattributes
git commit -m "chore: configure Git LFS for test disk images"
```

---

### Task 27: Dirty Volume Read-Only Enforcement Test

**Files:**
- Modify: `crates/libmnfts/tests/health_checks.rs`

Per H5: verify dirty volumes mount successfully in read-only mode (not rejected).

- [ ] **Step 1: Write enforcement test**

```rust
#[test]
fn test_dirty_volume_mounts_successfully_but_unhealthy() {
    let data = std::fs::read("tests/images/dirty.img").unwrap();
    let mut vol = NtfsVolume::open(Cursor::new(data));
    // Should succeed — dirty volumes mount, they just report unhealthy
    assert!(vol.is_ok(), "Dirty volume should mount successfully");
    let mut vol = vol.unwrap();
    assert!(!vol.is_healthy(), "Should report unhealthy");
    // Should still be able to list files
    let entries = list_directory(&mut vol, "/");
    assert!(entries.is_ok(), "Should be able to read files from dirty volume");
}
```

- [ ] **Step 2: Commit**

```bash
git commit -m "test: verify dirty volumes mount read-only with warning"
```

---

## Updated Dependencies

```
Task 1 → Task 2 → Task 3 → Task 4 → Task 5 (FSKit PoC) ← PHASE 0 EXIT GATE
  → Task 5b (NEW: resolve block device model)
    → Task 8 (Block I/O) → Task 8b (NEW: ntfs crate API audit)
      → Task 9 → Task 10 → Task 11 → Task 12 → Task 13
  → Task 6 (FFI e2e)

Task 7 (images) + Task 26 (LFS) — independent, do early

Task 14 (CLI) + Task E7 (emergency-unmount) — after Task 5
Task 15 (diagnostics) — after Task 9
Task 16 (watchdog, with E6 fixes) — after Task 5

Task 17 (tests) — after Tasks 10-12
Task 22 (corrupt MFT) — after Task 10
Task 23 (large files) — after Task 11
Task 24 (sparse/compressed) — after Task 10
Task 25 (fuzz) — after Task 9
Task 27 (dirty enforcement) — after Task 9

Task 18 (CI) — after Tasks 17, 25
Task 19 (docs) — after Task 14
Task 20 (Homebrew) — after Task 19
Task 21 (E2E) — after all above
```
