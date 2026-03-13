# mNFTS — Product & Technical Specification

**Version:** 1.0 (Draft)
**Date:** 2026-03-13
**Author:** mNFTS Core Team
**Status:** Approved for implementation planning

---

## 1. Executive Summary

**mNFTS** is a free, open-source NTFS driver for macOS built on Apple's FSKit framework. It provides native, safe, read/write access to NTFS-formatted external drives — starting with rock-solid read-only support and phasing in write operations carefully over time.

**Who it's for:** Mac users who plug in Windows-formatted external drives. Initially developers and power users; general Mac users once the product matures.

**Why it matters:** There is no good free option today. Apple's built-in NTFS support is read-only and unreliable. ntfs-3g requires FUSE, is unmaintained, and breaks on macOS upgrades. Commercial tools (Paragon, Tuxera) cost $20-80 and are closed-source black boxes. Reformatting to exFAT loses NTFS features and isn't always an option.

**What makes it better:**

- **FSKit-native** — no kernel extensions, no FUSE. Apple's sanctioned user-space filesystem path. Survives macOS upgrades.
- **Rust core** — memory-safe NTFS parsing. No buffer overflows corrupting your data.
- **Safety-first** — defaults to read-only when anything looks wrong. Explicit opt-in for writes. Crash watchdog forces clean unmount on failure.
- **Free and open-source** — MIT-licensed core. No subscriptions, no trials, no nag screens.

---

## 2. Product Scope

### 2.1 In-Scope by Version

| Version | Milestone | Features |
|---|---|---|
| **v0.1** | Read-only MVP | Mount NTFS volumes read-only via FSKit. CLI tool (`mnfts mount/unmount/status`). Basic file/folder reading. Unicode filenames. Finder visibility. Homebrew install. Crash watchdog. |
| **v0.5** | Limited Write | Safe write operations: create files, write new files, delete files, rename. Journal replay on mount. Forced read-only fallback on any inconsistency. CLI write-enable flag (`--rw`). Basic logging/diagnostics. |
| **v1.0** | Stable Release | Menu bar GUI app. Auto-mount on drive insertion. Notarized DMG installer. Modify existing files. Large file support (>4GB). Disk Utility visibility. Update notifications. Comprehensive diagnostics UI. |

### 2.2 Out of Scope

| Feature | Reason |
|---|---|
| NTFS compression (read/write) | Complex, high corruption risk, rarely needed on external drives |
| Encrypted volumes (BitLocker) | Entirely different problem domain |
| NTFS-to-NTFS copy with full metadata preservation | ACLs, security descriptors — Windows-specific semantics |
| Network drive / SMB NTFS | FSKit is for local block devices |
| Formatting drives as NTFS | Out of scope — use Windows or third-party tools |
| macOS versions < 15.4 | FSKit dependency, not negotiable |
| Internal drive mounting | Too dangerous for early versions, external only |

### 2.3 Success Criteria

- **v0.1:** Can `brew install mnfts`, plug in a Windows-formatted USB, and browse all files in Finder. Zero data corruption incidents.
- **v0.5:** Can safely create, rename, and delete files on NTFS volumes. Automatic fallback to read-only on any journal/metadata inconsistency.
- **v1.0:** Non-technical Mac user can install via DMG, plug in drive, and read/write files without ever opening Terminal.

### 2.4 Non-Goals

- Competing with Paragon/Tuxera on feature parity — we compete on trust, safety, and openness
- Full NTFS spec compliance — we support the common subset that real drives use
- Windows compatibility testing certification — we cross-validate but don't certify
- Performance parity with native HFS+/APFS — correctness over speed, always

---

## 3. User Stories

**US-1: Reading an NTFS drive**
> As a Mac developer, I plug in a colleague's Windows-formatted USB stick. I run `mnfts mount /dev/disk4s1` and the volume appears in Finder. I browse folders, open documents, and copy files to my Mac. When I eject, the volume unmounts cleanly.

**US-2: Writing to an NTFS drive**
> As a Mac user (v0.5+), I need to put files on a Windows-formatted USB for a friend. I mount with `mnfts mount --rw /dev/disk4s1`. I drag files into Finder. I see a menu bar indicator confirming write mode. I eject safely and my friend reads the files on Windows without issues.

**US-3: Power user / scripting**
> As a developer, I write a backup script that mounts an NTFS drive, rsyncs specific directories, and unmounts. I use `mnfts mount --rw --json /dev/disk4s1 /Volumes/Backup` for scriptable output. Exit codes are reliable (0 success, non-zero with specific codes for common failures).

**US-4: Debugging / diagnostics**
> As a contributor debugging a mount failure, I run `mnfts mount --log-level debug /dev/disk4s1`. I get structured logs showing MFT parsing, attribute resolution, and the exact point of failure. I run `mnfts inspect /dev/disk4s1` to dump volume metadata without mounting.

**US-5: Drive with errors**
> As a user, I plug in a drive that was unplugged from a Windows machine without safe-eject. mNFTS detects a dirty journal flag. It mounts read-only automatically and prints: `Warning: volume was not cleanly unmounted. Mounted read-only for safety. Use Windows chkdsk to repair, or pass --force-rw to override (risk of corruption).` I copy my files off safely.

**US-6: Crash recovery**
> As a user, the mNFTS process crashes mid-operation. The watchdog daemon detects the crash within 2 seconds, force-unmounts the volume, and writes a crash report to `~/Library/Logs/mNFTS/`. On next mount, mNFTS detects it was the last mounter, checks journal consistency, and mounts read-only if anything is suspect.

---

## 4. Technical Architecture

### 4.1 System Overview

```
┌─────────────────────────────────────────────────┐
│                   macOS                          │
│                                                  │
│  Finder / Apps                                   │
│       ↕                                          │
│  VFS (Virtual File System)                       │
│       ↕                                          │
│  fskitd  ←──────────────────────┐                │
│       ↕                         │                │
│  ┌──────────────────────┐  ┌────────────┐        │
│  │  mNFTS FSKit Module  │  │  Watchdog  │        │
│  │  (Swift shell)       │  │  (launchd) │        │
│  │       ↕              │  └────────────┘        │
│  │  libmnfts (Rust)     │                        │
│  │       ↕              │                        │
│  │  ntfs crate          │                        │
│  │       ↕              │                        │
│  │  Block device I/O    │                        │
│  └──────────────────────┘                        │
│       ↕                                          │
│  /dev/diskNsN (USB/SSD)                          │
└─────────────────────────────────────────────────┘

CLI: mnfts (Swift) ──→ XPC ──→ FSKit Module
GUI: mNFTS.app (Swift, menu bar) ──→ XPC ──→ FSKit Module
```

### 4.2 Components

| Component | Language | Role |
|---|---|---|
| `libmnfts` | Rust | NTFS parsing, read/write logic, journal handling, safety checks. The brain. |
| FSKit Module | Swift | Thin FSKit `FSUnaryFileSystem` subclass. Translates VFS ops → `libmnfts` C-ABI calls. |
| `mnfts` CLI | Swift | User-facing command-line tool. Communicates with FSKit module via XPC to trigger mount/unmount/status. |
| mNFTS.app | Swift | Menu bar app (v1.0). Auto-mount, status display, preferences. |
| Watchdog | Swift | launchd agent. Monitors FSKit extension health. Force-unmounts on crash. |

### 4.3 Block Device I/O Layer (`libmnfts::io`)

- Raw block reads/writes against `/dev/rdiskNsN` (character device for unbuffered I/O)
- Read-ahead buffer for sequential access patterns (configurable, default 1MB)
- All writes go through a single write gate that checks a `ReadWriteMode` enum — if it's `ReadOnly`, the write panics at the Rust level before any bytes hit disk
- Block cache with LRU eviction (in-process, no persistence)

### 4.4 NTFS Parsing Layer (`ntfs` crate + `libmnfts::ntfs`)

- `ntfs` crate handles: boot sector, MFT, attribute parsing, index B-trees, file name resolution, data runs
- `libmnfts::ntfs` extends with: write operations, journal interaction, safety validators
- All parsed structures are immutable Rust types — mutations create new copies (copy-on-write semantics at the data structure level)

### 4.5 Metadata Handling

- MFT records cached in-memory on first access, evicted under memory pressure
- File attribute resolution follows NTFS precedence: `$STANDARD_INFORMATION` → `$FILE_NAME` → `$DATA`
- Timestamps mapped to macOS conventions (NTFS uses Windows FILETIME, 100ns since 1601 → converted to `timespec`)
- File IDs: NTFS MFT reference numbers mapped to FSKit `FSItemID`

### 4.6 Read Path

```
Finder read() → VFS → fskitd → FSKit Module.read(itemID, offset, length)
  → libmnfts::read_file(mft_ref, offset, length)
    → resolve data attribute → walk data runs → read blocks → return bytes
```

No caching beyond the block cache — let macOS unified buffer cache handle upper-level caching.

### 4.7 Write Path (v0.5+)

```
Finder write() → VFS → fskitd → FSKit Module.write(itemID, offset, data)
  → libmnfts::check_write_gate()         // is RW mode active?
  → libmnfts::check_safety_invariants()   // is volume healthy?
  → libmnfts::journal_begin()             // log intent
  → libmnfts::execute_write()             // modify MFT + data runs + blocks
  → libmnfts::journal_commit()            // mark complete
```

Every write is journaled. If any step fails, the journal entry is uncommitted and the volume is flipped to read-only immediately.

### 4.8 Mount Management

- `mnfts mount` → discovers device → validates NTFS boot sector → opens block device → registers with FSKit → volume appears in `/Volumes/`
- `mnfts unmount` → flushes all caches → writes journal → closes block device → deregisters from FSKit
- Auto-mount (v1.0): DiskArbitration framework callback on device insertion → trigger FSKit mount

### 4.9 Journaling / Crash-Safety Strategy

- On mount: read NTFS `$LogFile`. If dirty, refuse write mode (mount read-only).
- On write: use NTFS's own journal (`$LogFile`) for redo/undo records before any metadata write.
- On unmount: flush journal, mark volume clean.
- On crash: watchdog force-unmounts. Next mount sees dirty journal → read-only.
- **v0.1 does not write to `$LogFile` at all** — it's read-only, so no journal interaction beyond checking the dirty flag.

### 4.10 Permissions and Security Model

- FSKit extension runs as a non-root user (FSKit's sandboxed execution model)
- Block device access granted by macOS disk arbitration — user must approve the device
- No `sudo` required for normal operation
- Write mode requires explicit opt-in (CLI flag or GUI toggle) — never default-on
- Files presented with macOS-friendly permissions: owner = mounting user, mode = 755 dirs / 644 files (NTFS ACLs not mapped, just ignored)

### 4.11 Logging / Diagnostics

- Structured logging via Rust `tracing` crate, forwarded to `os_log` via the Swift bridge
- Log levels: error (always), warn (default), info, debug, trace
- `mnfts inspect <device>` — dumps volume metadata (boot sector, MFT summary, journal state) without mounting
- Crash reports written to `~/Library/Logs/mNFTS/`
- `mnfts doctor <device>` — runs read-only health checks and reports issues

### 4.12 CLI / GUI Relationship

- CLI and GUI are independent frontends to the same FSKit module
- No shared state between them — FSKit module is the single source of truth
- GUI wraps the same XPC calls the CLI makes, plus DiskArbitration callbacks for auto-mount
- Both can be installed independently; GUI is not required

### 4.13 Update Strategy

- Homebrew: standard `brew upgrade mnfts`
- DMG: Sparkle framework for update notifications (v1.0). No auto-update — user triggers install.
- No telemetry, no phoning home. Version check is a simple GitHub releases API call, opt-in only.

---

## 5. Implementation Strategy

### 5.1 Build vs. Reuse

| Component | Strategy | Rationale |
|---|---|---|
| NTFS parsing (MFT, attributes, B-trees, data runs) | **Reuse** — `ntfs` crate | Pure Rust, MIT, no unsafe, well-structured. Covers 90% of read-path needs. |
| NTFS write operations | **Build** — extend `ntfs` or fork | The crate is read-only today. Write support is new code. |
| NTFS journal (`$LogFile`) interaction | **Build** | No existing Rust implementation. Reference ntfs-3g for format understanding, implement fresh in Rust. |
| Block I/O layer | **Build** | Thin, project-specific. ~500 lines. |
| FSKit integration | **Build** | No existing FSKit filesystem examples in open source. Apple's sample code + WWDC sessions are the reference. |
| Swift-Rust FFI bridge | **Build** | C-ABI boundary via `cbindgen`. Well-understood pattern. |
| CLI | **Build** | Swift with `ArgumentParser`. |
| Watchdog | **Build** | ~100-200 lines. launchd plist + Swift process monitor. |
| GUI (v1.0) | **Build** | SwiftUI menu bar app. |

### 5.2 What to Reference but NOT Copy

- **ntfs-3g source** — read to understand NTFS write semantics, journal format, and edge cases. Do NOT port C code. ntfs-3g is GPL — even reading it requires discipline to avoid creating a derivative work.
- **Linux kernel NTFS3 driver** — useful for understanding write ordering and journal replay. GPL-licensed, same caution.
- **Microsoft's published NTFS documentation** — official spec is partial but covers on-disk structures.

### 5.3 Dangerous Shortcuts to Avoid

| Shortcut | Why it's dangerous | Do instead |
|---|---|---|
| Wrapping ntfs-3g in a shim | GPL contaminates MIT. C code undermines safety. | Clean-room Rust implementation. |
| Skipping journal writes for "simple" operations | One crash during unjournaled write corrupts the volume. | Journal every metadata mutation. No exceptions. |
| Treating NTFS as "just FAT with extra steps" | Complex B-tree indexes, MFT self-referencing, attribute nesting. | Invest time understanding MFT and attribute model deeply. |
| Using `unsafe` Rust for performance | Defeats the safety proposition. | Zero `unsafe` in NTFS code. Only at FFI boundary and block I/O syscalls. Each `unsafe` block requires a `// SAFETY:` comment. |
| Copying on-disk structures with `repr(C)` and raw casts | Endianness bugs, alignment issues, UB. | Explicit deserialization (the `ntfs` crate already does this). |

---

## 6. macOS Integration

### 6.1 Mount/Unmount

- v0.1: Manual via `mnfts mount /dev/diskNsN`. Volume appears at `/Volumes/<VolumeLabel>`. Unmount via `mnfts unmount` or Finder eject.
- v1.0: DiskArbitration callback detects NTFS partition on USB insertion → auto-mounts via FSKit. Standard Finder sidebar appearance.
- Eject: FSKit handles unmount signaling. `libmnfts` flushes caches, writes journal, closes device. If files are open, macOS shows the standard "disk in use" dialog.

### 6.2 Finder Visibility

- FSKit volumes appear natively in Finder — no hacks. Finder treats FSKit volumes as first-class.
- Quick Look, Spotlight indexing all work because macOS treats FSKit volumes natively.
- Read-only volumes get the standard lock badge automatically.

### 6.3 Disk Utility

- FSKit-registered filesystems appear in Disk Utility's volume list.
- We do NOT register as a formatter — Disk Utility's "Erase" won't offer NTFS.

### 6.4 Permissions / Sandboxing

- FSKit extensions require user consent on first use.
- No Full Disk Access or special entitlements needed.
- No `sudo` for normal operations.
- FSKit module runs in Apple's extension sandbox — limited to filesystem operations on the granted device. No network access, no file access outside the block device.

### 6.5 Installer

- v0.1: `brew install mnfts` with post-install FSKit extension registration.
- v1.0: Notarized `.dmg` with `mNFTS.app`. First launch installs system extension via macOS standard approval dialog.
- Uninstall: `brew uninstall` or drag to Trash. Extension deregistered automatically.

### 6.6 Native Feel

- No preference panes, no kernel extension approval, no reboot.
- Menu bar app (v1.0): monochrome icon, standard menu, right-click to quit.
- Error dialogs use `NSAlert`.
- Logs go to unified logging (`os_log`) — visible in Console.app.

---

## 7. Safety Model

### 7.1 Core Principle

**When in doubt, refuse to write. A user who can't write to their drive is annoyed. A user whose drive is corrupted is furious and never comes back.**

### 7.2 Safety Invariants

| # | Invariant | Enforcement |
|---|---|---|
| S1 | No write shall occur when the volume is in read-only mode | Rust type-state pattern. Code that writes cannot compile against a read-only handle. |
| S2 | No metadata write shall occur without a journal entry | Write path requires a `JournalTransaction` token as a function parameter — can't call without one. |
| S3 | A dirty volume is always mounted read-only | `$LogFile` dirty flag checked on mount. If dirty → read-only. Override requires `--force-rw` plus confirmation. |
| S4 | A crash always results in a safe state | Watchdog force-unmounts within 2 seconds. Next mount detects dirty journal → read-only. |
| S5 | Unsupported NTFS features never block read access | Unknown attributes are skipped, not errored. Files with unsupported features are visible but may show degraded metadata. |
| S6 | Write mode is never the default | CLI requires `--rw`. GUI requires explicit toggle. First-time users always get read-only. |

### 7.3 Forced Read-Only Conditions

| Condition | Action |
|---|---|
| `$LogFile` dirty flag set | Read-only. Warn user to run `chkdsk` on Windows. |
| `$Volume` indicates Windows hibernation | Read-only. Writing would corrupt hibernation state. |
| MFT parsing encounters unexpected structure | Read-only. Log details. |
| Journal replay fails or is incomplete | Read-only. |
| Block device reports I/O errors | Read-only. Warn about possible hardware failure. |
| NTFS version > 3.1 | Read-only. Unknown format. |
| Unsupported features active (dedup, storage tiers) | Read-only. |
| Watchdog previously detected crash for this volume | Read-only until user explicitly re-enables. |

### 7.4 Safe Write Operations (v0.5)

| Operation | Why it's safe |
|---|---|
| Create new file (empty) | Allocates MFT record + updates parent index. Well-understood, bounded change. |
| Write data to new file | Allocates data runs, writes clusters. No existing data at risk. |
| Delete file | Marks MFT record as free, updates parent index, frees clusters. Reversible if journal intact. |
| Rename file (same directory) | Updates `$FILE_NAME` attribute and parent index entry. Single-directory scope. |
| Create directory | Same as create file, with index root attribute. |

### 7.5 Operations Blocked Until Later

| Operation | Risk | Earliest version |
|---|---|---|
| Modify existing file contents | Partial overwrites, data run splitting, cluster reallocation | v0.7 |
| Move file across directories | Two index updates must be atomic | v0.7 |
| Truncate / extend existing file | Data run manipulation, sparse/compressed extents | v0.7 |
| Set timestamps / attributes | Low risk but low value early | v0.7 |
| Modify large files (>4GB) | Multi-extent data runs, MFT attribute lists | v1.0 |

### 7.6 Failure Handling

**Unclean shutdown (power loss, cable pull):**
- Journal contains redo/undo records for in-flight metadata writes.
- Next mount: dirty journal → read-only.
- v0.5 does NOT replay the journal. User must run `chkdsk` or accept read-only. Journal replay is v1.0+.

**Bad sectors:**
- Block I/O layer reports errors up the stack.
- NTFS layer marks affected files as unreadable, continues mounting remaining files.
- `mnfts doctor` reports affected files.
- Never write to a volume with known bad sectors.

**Inconsistent metadata:**
- MFT record number mismatch → skip record, log, continue.
- Index B-tree cycle or invalid pointer → stop traversal, mount read-only, report via `mnfts doctor`.
- Attribute length exceeds record bounds → skip attribute, log, continue.

**Partially supported features:**
- Compressed files: visible but return `ENOTSUP` on read.
- Encrypted files (EFS): visible, unreadable, clear error.
- Alternate data streams: ignored. Default `$DATA` stream only.
- Reparse points / symlinks: show as regular files in v0.1, resolve in v0.7.

### 7.7 Emergency Unmount

`mnfts emergency-unmount <device>` — immediately closes block device, discards all caches, deregisters from FSKit. No flush, no journal write. For when something has gone wrong and the user just wants the drive disconnected.

---

## 8. NTFS Feature Support Matrix

| Feature | Read | Write | Risk | Version |
|---|---|---|---|---|
| Basic files and folders | Yes | Create/delete v0.5, modify v0.7 | Low | v0.1 read, v0.5 write |
| Unicode filenames (UTF-16) | Yes | v0.5 | Low | v0.1 |
| Long filenames + 8.3 short names | Yes (both) | Generate 8.3 on create v0.5 | Medium | v0.1 read |
| Rename (same directory) | — | v0.5 | Low | v0.5 |
| Rename (cross-directory move) | — | v0.7 | Medium | v0.7 |
| Delete files | — | v0.5 | Low | v0.5 |
| Delete non-empty directories | — | v0.7 | Medium | v0.7 |
| Large files (>4GB) | Yes | v1.0 | Medium | v0.1 read, v1.0 write |
| Sparse files | Detect, read allocated regions | Never | High | v0.1 read-only |
| NTFS compression | Detect, flag as unreadable | Never | Very High | v0.1 detection only |
| Alternate data streams (ADS) | Ignored (default stream only) | Never | Medium | Post-v1.0 if ever |
| ACL / security descriptors | Ignored (map to Unix perms) | Never | High | Never |
| Reparse points | Regular file v0.1, resolve v0.7 | Never | High | v0.7 read |
| Symlinks (NTFS) | Regular file v0.1, resolve v0.7 | v1.0 | Medium | v0.7 read, v1.0 write |
| Hard links | Read v0.1 | v1.0 | Medium | v0.1 read, v1.0 write |
| Extended attributes ($EA) | Ignored | Never | Low | Post-v1.0 if ever |
| Case sensitivity | Case-preserving, case-insensitive | Same | Low | v0.1 |
| Timestamps (create/modify/access) | Yes (mapped to macOS) | Set on create v0.5 | Low | v0.1 read, v0.5 write |
| `$LogFile` — dirty check | Yes | — | Low | v0.1 |
| `$LogFile` — write journal entries | — | v0.5 | High | v0.5 |
| `$LogFile` — replay journal | — | — | Very High | v1.0 |
| `$Bitmap` (cluster allocation) | Read | Update on write v0.5 | Medium | v0.1 read, v0.5 write |
| `$UpCase` (case folding table) | Read | Never | Low | v0.1 |
| `$Volume` flags | Read, enforce | Never | Low | v0.1 |
| Damaged volume (corrupt MFT) | Partial mount, skip bad records | Never write to damaged volumes | High | v0.1 |
| Damaged volume (corrupt index) | Fallback to MFT scan | Never write | High | v0.5 |

**Legend:**
- **Never** = architectural decision, not a roadmap item.
- **Risk** = risk of data corruption if the implementation has a bug.

---

## 9. Development Roadmap

### Phase 0: Discovery & Research (2 weeks)

**Deliverables:**
- Working FSKit "hello world" — mount a dummy read-only filesystem via FSKit on macOS 15.4
- Rust-Swift FFI prototype — Swift calls Rust function, confirmed working in app extension context
- `ntfs` crate evaluation — mount a real NTFS USB image, enumerate files, read contents
- Document FSKit's actual behavior (poorly documented — expect surprises)

**Risks:** FSKit may have undocumented limitations or bugs.

**Exit criteria:** Can read a file from an NTFS disk image through FSKit into Finder using Rust code bridged via Swift.

**AI assists:** FSKit API exploration, FFI boilerplate, `ntfs` crate usage examples, research log.

### Phase 1: Read-Only MVP / v0.1 (4 weeks)

**Deliverables:**
- `libmnfts` Rust library: mount, enumerate dirs, read files, Unicode, volume health validation
- FSKit module: full read-only filesystem in Finder
- `mnfts` CLI: `mount`, `unmount`, `status`, `inspect`, `doctor`
- Crash watchdog launchd agent
- Homebrew formula
- Test suite: unit tests, integration tests against golden NTFS images
- README, LICENSE (MIT), CONTRIBUTING guide

**Risks:** FSKit edge cases, unusual NTFS volumes, block device permission model.

**Exit criteria:**
- Mount 10 different real-world NTFS drives. All files readable.
- Zero crashes in 48 hours of continuous mount.
- `brew install mnfts && mnfts mount /dev/diskNsN` works end-to-end.

**AI assists:** CLI scaffolding, FSKit boilerplate, test harness, Homebrew formula, documentation.

### Phase 2: Internal Alpha (2 weeks)

**Deliverables:**
- Dogfood on every NTFS drive available
- Edge case collection, performance data, crash logs
- Bug fixes, hardened error handling
- GitHub repo published

**Risks:** Real-world drives surface unexpected parsing edge cases.

**Exit criteria:** Daily use for 2+ weeks on 5+ volumes. All bugs resolved or documented.

**AI assists:** Bug triage, regression tests, documentation.

### Phase 3: Limited Write Alpha / v0.5 (6 weeks)

**Deliverables:**
- Write gate infrastructure (type-state pattern)
- Journal write support (`$LogFile`)
- Create file, write new file, delete file, rename (same dir), create directory
- Forced read-only fallback on inconsistency
- `--rw` CLI flag
- Cross-validation with Windows `chkdsk`
- Power-loss simulation tests
- Property-based tests for MFT allocation and B-tree updates

**Risks:** Most dangerous phase. Journal format, B-tree index updates.

**Exit criteria:**
- Create 1000 files, delete 500, rename 200 — `chkdsk` passes after each batch.
- Simulated power loss at every write step — volume always recoverable.
- Zero corruption across 100 test cycles.

**AI assists:** Test generation, property-based tests, cross-validation scripts. **AI must NOT write journal or B-tree code without human review.**

### Phase 4: Public Beta / v0.5 Release (3 weeks)

**Deliverables:**
- Public Homebrew release
- GitHub Discussions
- Bug bounty for corruption bugs
- Monitoring: opt-in structured crash reports
- Documentation: troubleshooting, known limitations, FAQ

**Risks:** Public users find edge cases.

**Exit criteria:** 50+ users, no corruption reports, <48hr response on critical bugs.

**AI assists:** Documentation, FAQ, issue triage, reproduction test cases.

### Phase 5: Stable v1.0 (9 weeks)

**Deliverables:**
- Modify existing files, cross-directory move, large file writes
- Journal replay on mount
- SwiftUI menu bar GUI
- Auto-mount via DiskArbitration
- Notarized DMG
- Sparkle update framework
- Diagnostics UI
- Disk Utility visibility
- Fuzzing, golden images, cross-platform validation matrix
- Website and user documentation

**Risks:** GUI work is different skillset. Journal replay is complex. Notarization overhead.

**Exit criteria:**
- Non-technical user can install DMG, plug in drive, read/write without Terminal.
- `chkdsk` passes after 1000 mixed operations.
- 6 months of public beta with no corruption reports.

**AI assists:** SwiftUI, installer automation, docs, website. **AI must NOT write journal replay unsupervised.**

### Total: ~26 weeks (~6 months) full-time with AI assistance.

---

## 10. Team Plan

### Roles

| Role | Solo phase (you) | First to offload | When to recruit |
|---|---|---|---|
| macOS/FSKit engineer | You — core skill | Never | When a contributor has FSKit/system extension experience |
| Filesystem engineer | You + `ntfs` crate | AI generates tests, you review write-path | When write support gets complex (v0.5+) |
| QA/test engineer | You + AI test suites | **First role to share.** Contributors can write tests. | After v0.1. "Test on your drives" = easiest on-ramp. |
| Release engineer | You. Homebrew, CI/CD. | AI scaffolds CI config | Not critical until v1.0 |
| Docs/community | You + AI drafts | **Second role to share.** | After v0.5 |
| Legal/research advisor | **Gap.** One-time consult needed. | Immediately — before writing NTFS write code. | Never a full role. 2-3 hours with an IP attorney. |

### Critical Contributor Profile

If you attract one contributor, prioritize: someone who has written filesystem code, understands journaling/crash recovery, and reads Rust. This person reviews write-path code.

### Contributor On-Ramp

| Type | Difficulty | New contributor friendly | When |
|---|---|---|---|
| Test on your hardware | Trivial | Yes | v0.1 |
| Add golden disk images | Easy | Yes | v0.1 |
| Write integration tests | Easy-Medium | Yes | v0.1 |
| CLI UX, error messages | Medium | Yes | v0.5 |
| Documentation | Medium | Yes | v0.5 |
| GUI improvements | Medium | Needs Swift | v1.0 |
| NTFS read-side features | Hard | Needs Rust + NTFS | v0.5 |
| Write-path code | Very Hard | Deep review required | v0.5+ |

### Legal Consult (Phase 0)

One session covering:
- NTFS patent landscape (Microsoft patents on NTFS features)
- Clean-room concerns with ntfs-3g (GPL)
- MIT license suitability for a filesystem driver
- Microsoft open-specification program obligations

---

## 11. Testing Strategy

### Test Pyramid

```
         ╱╲
        ╱  ╲       Fuzzing / Power-loss simulation (5%)
       ╱    ╲
      ╱──────╲
     ╱        ╲     Cross-validation with Windows (10%)
    ╱          ╲
   ╱────────────╲
  ╱              ╲   Integration tests (25%)
 ╱                ╲
╱──────────────────╲
╱                    ╲ Unit tests (60%)
╱────────────────────────╲
```

### Unit Tests (Rust)

| Area | Approach |
|---|---|
| MFT parsing | Golden byte buffers, extend `ntfs` crate tests |
| Data run decoding | Table-driven tests with known byte sequences |
| B-tree index | Property-based: any insert/delete sequence → valid tree |
| Cluster allocation | Property: bitmap always consistent with allocation |
| Journal entries | Roundtrip: create → serialize → deserialize → compare |
| Write gate | Compile-time test: write via read-only handle fails to compile |
| Unicode | Known edge-case corpus (NFD/NFC, surrogates, null) |

**Target: 80%+ coverage on `libmnfts`.**

### Integration Tests

| Test | Validates |
|---|---|
| Mount → list root → unmount | Full stack end-to-end |
| Mount → read file → compare SHA256 | Data integrity |
| Mount → browse deep path (>20 levels) | Path resolution |
| Mount dirty volume → verify read-only | Safety model |
| Mount → write → unmount → remount → verify (v0.5) | Write persistence |
| CLI on dirty volume → verify rejection | Safety CLI enforcement |
| CLI `doctor` healthy vs corrupt → verify output | Diagnostics accuracy |

### Golden Disk Images

| Image | Purpose |
|---|---|
| `win10-basic.img` | Standard files, folders, Unicode |
| `win11-basic.img` | Same on Win 11 |
| `large-files.img` | Files >4GB, many extents |
| `deep-paths.img` | 50+ level nesting |
| `dirty-journal.img` | Unclean shutdown |
| `hibernated.img` | Windows hibernation flag |
| `fragmented.img` | Heavily fragmented volume |
| `special-chars.img` | Emoji, CJK, RTL, zero-width chars |
| `compressed.img` | NTFS-compressed files |
| `sparse.img` | Sparse files |
| `corrupt-mft.img` | Damaged MFT records |
| `corrupt-index.img` | Damaged directory index |
| `empty.img` | Freshly formatted |
| `nearly-full.img` | <1% free space |

### Property-Based Tests

| Property | Invariant |
|---|---|
| Allocation roundtrip | Allocate N → free N → bitmap unchanged |
| B-tree consistency | Any op sequence → valid sorted tree |
| MFT roundtrip | Serialize → deserialize → serialize = identical |
| Journal roundtrip | Begin → write → commit → replay = intended state |
| Dir listing completeness | Listed files = MFT records with that parent |

### Fuzzing

| Target | Tool | Finds |
|---|---|---|
| MFT record parser | `cargo-fuzz` | Panics, infinite loops, OOM |
| Data run decoder | `cargo-fuzz` | Out-of-bounds reads |
| Index B-tree traversal | `cargo-fuzz` | Cycles, invalid pointers, stack overflow |
| Boot sector parser | `cargo-fuzz` | Malformed volume crashes |

Run continuously in CI. 10 minutes per PR minimum.

### Power-Loss Simulation

| Test | Method |
|---|---|
| Kill mid-write | `kill -9` FSKit extension. Verify watchdog unmounts. Next mount read-only. `chkdsk` passes. |
| Crash at every journal step | Inject crash points via `#[cfg(test)]` hooks. Verify recovery at each point. |
| Simulated I/O failure | Mock block device returning errors after N writes. Verify read-only degradation. |

**Not optional. Every write-path change must pass.**

### Cross-Validation with Windows

| Check | Method |
|---|---|
| `chkdsk /f` passes | After every write test run, `chkdsk` in Windows VM |
| Files readable on Windows | Write via mNFTS → verify on Windows (SHA256) |
| Windows files readable via mNFTS | Create on Windows → read via mNFTS (SHA256) |
| Bidirectional stress | Alternate 100-file batches Mac↔Windows, 10 rounds |

Automate via Windows runner in GitHub Actions CI.

### Performance Benchmarks

| Benchmark | Metric |
|---|---|
| Mount time (10k files) | Seconds to Finder-ready |
| Sequential read | MB/s on 1GB file |
| Random read latency | ms for 4KB reads |
| Directory listing (10k entries) | Seconds |
| Write throughput (v0.5+) | MB/s sequential |

Run on every release, not every PR.

### CI Matrix

| Axis | Values |
|---|---|
| macOS | 15.4, latest beta |
| Architecture | Apple Silicon (primary), Intel (if available) |
| Rust toolchain | stable, nightly (fuzzing) |
| NTFS images | Full golden suite |

---

## 12. AI-Assisted Development Plan

### What AI Accelerates

| Area | Time savings |
|---|---|
| FSKit boilerplate | 70-80% |
| Rust-Swift FFI bridge | 60-70% |
| CLI scaffolding | 80% |
| Test generation | 70% |
| CI/CD pipelines | 80% |
| Documentation | 70% |
| SwiftUI menu bar app | 70% |
| Homebrew formula | 90% |
| Windows VM test scripts | 80% |
| Bug reproduction tests | 50% |

### What AI Must NOT Do Unsupervised

| Area | Why | Policy |
|---|---|---|
| NTFS journal write logic | Wrong byte in `$LogFile` = unrecoverable volume | Every line human-reviewed |
| B-tree index mutation | Must maintain exact NTFS invariants | Human-reviewed + property tests |
| Cluster allocation/deallocation | Double-free corrupts bitmap | Human-reviewed + roundtrip tests |
| MFT record write-back | Corrupt MFT = everything gone | Human-reviewed, no exceptions |
| Crash recovery paths | Last line of defense must be correct | Human-reviewed + power-loss sim |
| Security-sensitive code | Permissions, entitlements, sandbox | Human-reviewed |

### Module AI Suitability

```
Safe for AI scaffolding       Needs human review         Human-only
──────────────────────────────────────────────────────────────────────
CLI commands                   Read path integration      Journal writes
FSKit delegate stubs           MFT attribute resolution   B-tree mutations
FFI bridge functions           Block cache eviction       Cluster bitmap updates
Test harnesses                 Unicode normalization      Crash recovery
CI config                      Error handling paths       Safety invariant code
GUI views                      Watchdog logic             Write gate type-state
Documentation                  Mount/unmount sequencing   Power-loss handling
Homebrew formula               Dirty flag checking        MFT record serialization
Logging infrastructure         Data run decoding
```

### Code Review Tiers

**Tier 1 — Trust but verify (boilerplate, docs, tests, CI):**
AI generates, you skim. Merge if tests pass. 2-5 min review.

**Tier 2 — Review every line (read path, integration, CLI):**
AI drafts, you read every line, test manually, refactor. 15-30 min per module.

**Tier 3 — AI assists, human writes (write path, journal, crash safety):**
You write the code. AI helps with tests, suggestions, spec explanations, code review. You never paste AI write-path code directly. As long as it takes.

### AI Strengths for This Project

- Reading and explaining the NTFS spec
- Generating corrupt disk images (hex-edit scripts)
- Writing regression tests from bug reports
- Reviewing code for NTFS spec compliance

### AI Weaknesses for This Project

- Reasoning about crash semantics ("what if power loss between step 3 and 4?")
- NTFS edge cases not in training data
- FSKit specifics (too new, thin training data)

---

## 13. Open-Source Strategy

### License

**MIT.** Simple, permissive, contributor-friendly. Compatible with the `ntfs` crate (MIT). If the Phase 0 legal consult reveals patent concerns, revisit.

### Repo Structure

```
mnfts/
├── crates/
│   ├── libmnfts/          # Rust core library
│   │   ├── src/
│   │   │   ├── io/        # Block device I/O
│   │   │   ├── ntfs/      # NTFS write extensions
│   │   │   ├── journal/   # Journal read/write
│   │   │   ├── safety/    # Write gate, invariant checks
│   │   │   └── ffi/       # C-ABI exports
│   │   └── Cargo.toml
│   └── mnfts-fuzz/        # Fuzz targets
├── swift/
│   ├── FSKitModule/       # FSKit extension
│   ├── CLI/               # mnfts CLI
│   ├── Watchdog/          # launchd crash monitor
│   └── App/               # Menu bar GUI (v1.0)
├── tests/
│   ├── images/            # Golden NTFS disk images (LFS)
│   ├── integration/       # End-to-end tests
│   └── cross-validation/  # Windows VM scripts
├── docs/
│   ├── superpowers/specs/ # Design specs
│   ├── architecture.md
│   └── ntfs-notes.md
├── .github/
│   ├── workflows/         # CI/CD
│   ├── ISSUE_TEMPLATE/
│   └── SECURITY.md
├── Formula/               # Homebrew formula
├── CLAUDE.md
├── LICENSE
├── README.md
└── CONTRIBUTING.md
```

### Governance

Benevolent dictator (you) through v0.5. After v1.0 with consistent contributors, lightweight maintainer model: 2 maintainers required for write-path PRs. `MAINTAINERS.md` listing merge authority.

### Security Disclosure

Email-based responsible disclosure. 48hr acknowledgment. 7-day patch target for corruption bugs. Credit in release notes.

### Building Trust

1. Every design decision documented. Every bug public. Every limitation stated.
2. No silent failures — always explain why something happened.
3. Conservative defaults — read-only always, write requires opt-in.
4. Published `chkdsk` validation results in release notes.
5. Corruption = P0 — highest priority, postmortem published.
6. No telemetry, no analytics, no phoning home.

---

## 14. Risk Register

| # | Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|---|
| R1 | Data corruption from write-path bug | Critical | Medium | Type-state write gate. Journal every mutation. Power-loss simulation. Conservative op whitelist. Default read-only. |
| R2 | FSKit too new / buggy / under-documented | High | High | Phase 0 dedicated to FSKit PoC. Fallback: macFUSE adapter. Rust core stays FSKit-agnostic. |
| R3 | Microsoft NTFS patents | High | Low-Med | Legal consult Phase 0. NTFS is 30+ years old. Defer patented features. |
| R4 | Clean-room contamination from ntfs-3g (GPL) | High | Low | Policy: read for understanding, never copy. `ntfs` crate is MIT, independently written. |
| R5 | NTFS edge cases cause silent data loss | Critical | Medium | Golden images. `chkdsk` cross-validation. Fuzzing. Property tests. |
| R6 | macOS update breaks FSKit | High | Medium | CI on latest beta. FSKit is Apple's strategic direction. |
| R7 | User trust — "free NTFS driver" sounds sketchy | Medium | High | Open source. No admin for unclear reasons. No telemetry. Published validation. |
| R8 | Support burden overwhelms solo maintainer | Medium | High | Conservative scope. GitHub Discussions. Clear known-limitations. Issue templates. |
| R9 | Performance too slow for large copies | Medium | Low-Med | USB/drive speed is bottleneck, not driver. Benchmark in Phase 1. |
| R10 | Installer / extension approval friction | Medium | Medium | v0.1 Homebrew = no friction. v1.0 uses standard macOS extension approval. |
| R11 | `ntfs` crate bugs or missing features | Medium | Medium | Contribute fixes upstream. Fork if needed. Fuzzing finds bugs. |
| R12 | NTFS version compatibility | Low | Low | NTFS 3.1 only (XP through Win11). Refuse others. |
| R13 | Contributor submits buggy write-path code | High | Low-Med | Maintainer review required. CI power-loss sim + `chkdsk`. No auto-merge. |
| R14 | Project abandoned / burnout | Medium | Medium | Ship small. Each version independently useful. Architecture docs for handoff. |

### Priority Matrix

```
                    Low Likelihood    Medium           High
                 ┌─────────────────┬────────────────┬────────────────┐
    Critical     │                 │ R1, R5         │                │
                 ├─────────────────┼────────────────┼────────────────┤
    High         │ R3, R4          │ R2, R6, R13    │ R7, R8         │
                 ├─────────────────┼────────────────┼────────────────┤
    Medium       │ R12             │ R9, R10, R11   │ R14            │
                 └─────────────────┴────────────────┴────────────────┘
```

---

## 15. Competitive Positioning

| Solution | Price | Write | Architecture | macOS update survival | Trust |
|---|---|---|---|---|---|
| Paragon NTFS | $20/yr or $50 | Full | Kext → system ext | Frequently breaks | Closed source |
| Tuxera NTFS | $15-80 | Full | Kext | Breaks | Closed source |
| ntfs-3g + macFUSE | Free | Full (slow) | FUSE | Breaks, SIP issues | Unmaintained |
| Mounty | Free | Hack | Apple's hidden flag | Fragile | Undocumented behavior |
| Reformat exFAT | Free | N/A | N/A | N/A | Destructive |
| **mNFTS** | **Free** | **Phased** | **FSKit** | **By design** | **Open, auditable** |

### Compete on:
- Free vs. paid
- Open source vs. black box
- FSKit-native vs. legacy kext/FUSE
- Safety and transparency vs. "trust our binary"

### Don't compete on:
- Feature completeness (tell users to buy Paragon if they need everything now)
- Enterprise/fleet management
- Maximum write performance
- BitLocker

### Honest Early Weaknesses (publish in README):
- v0.1: Read-only, CLI only, no compressed/encrypted files
- v0.5: Limited writes (new files only), no journal replay, slower than commercial
- v1.0: No compression writes, no BitLocker, no formatting, user-space performance ceiling

### Positioning:
> mNFTS is the NTFS driver for people who want to trust what's touching their data. Free, open-source, built on Apple's modern filesystem framework. It earns write access one proven operation at a time.

---

## 16. Recommended v0.1 Definition

### Exact Scope

**Ships:** `mnfts` CLI, `libmnfts` Rust core, FSKit system extension, crash watchdog, Homebrew formula.

**Does:**
- `mnfts mount /dev/diskNsN` → volume in Finder, read-only
- `mnfts unmount`, `status`, `inspect`, `doctor`
- Browse, copy, open, Quick Look
- Unicode, long paths, large files, hard links
- Dirty/hibernation detection → read-only with warning
- Corrupt MFT → skip and continue
- Watchdog → force unmount on crash

**Does NOT:** Write anything. Auto-mount. GUI. Compressed/encrypted/ADS.

### Safest Architecture

```
USB → /dev/rdiskNsN (O_RDONLY) → libmnfts (Rust) → FSKit (Swift) → Finder
                                                          ↑
                                                     Watchdog monitors
```

No write path exists in the code. Block device opened read-only at the fd level. Defense in depth.

### First 90 Days

**Days 1-14:** FSKit PoC + Rust-Swift FFI + `ntfs` crate integration
**Days 15-45:** Core read path, real USB drives, error handling, CLI
**Days 46-65:** Watchdog, full CLI, test suite (80%+ coverage), fuzzing
**Days 66-90:** Homebrew formula, dogfood, bug fixes, public release

### Biggest Thing to Avoid

**Do not start write support before v0.1 is rock-solid.** Ship read-only. Let real users test it. When bug reports slow to a trickle, then start writes. Not before.

---

## Appendix: One-Page Actionable Summary

**What:** mNFTS — free, open-source, FSKit-native NTFS driver for macOS 15.4+.

**Stack:** Rust core (`ntfs` crate) + Swift FSKit module + Swift CLI. Crash watchdog. MIT license.

**v0.1 (ship in 90 days):** Read-only mount of NTFS USB/SSD drives. CLI only. Homebrew.

**Architecture:** Rust `libmnfts` → C-ABI → Swift FSKit `FSUnaryFileSystem` → Finder. No kext, no FUSE, no sudo.

**Safety:** Read-only default. Type-state write gate. Journal every mutation. Watchdog force-unmount on crash. Dirty volumes always read-only.

**Write support timeline:** v0.5 (~4 months): create/delete/rename. v1.0 (~6 months): modify, large files, GUI, auto-mount.

**Day 1 actions:**
1. Create repo with Rust workspace + Swift package
2. Build FSKit hello-world extension
3. Schedule legal consult for NTFS patents + GPL clean-room
4. Download/create first golden NTFS disk image

**Biggest risk:** FSKit maturity (mitigate in Phase 0). Data corruption (mitigate with testing discipline).

**Biggest mistake to avoid:** Starting write support before read-only is battle-tested.
