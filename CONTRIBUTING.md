# Contributing to mNFTS

Thank you for your interest in contributing to mNFTS. This guide covers everything you need to get started.

## Build Prerequisites

- **Rust stable toolchain** (install via [rustup](https://rustup.rs))
- **Xcode 16+** (for Swift compilation and FSKit headers)
- **macOS 15.4+** (required for FSKit APIs)

## Building

```bash
make build
```

This builds the Rust core library (libmnfts) and the Swift CLI.

## Testing

```bash
make test       # Run all tests (Rust + Swift)
cargo test      # Run Rust tests only
```

Integration tests use golden NTFS disk images located in `tests/images/`. See `tests/images/README.md` for details on generating and managing test images.

## Linting

```bash
make lint
```

This runs `cargo fmt --check` and `cargo clippy` to enforce formatting and catch common issues.

## Code Style

- **Formatting**: Always run `cargo fmt` before committing.
- **Linting**: Code must pass `cargo clippy` with no warnings.
- **Immutable patterns**: Prefer creating new objects over mutating existing ones. Avoid in-place mutation wherever possible.
- **Small files**: Keep files focused and under 800 lines. Extract utilities into separate modules when files grow large.
- **Error handling**: Handle errors explicitly at every level. Never silently swallow errors.

## Pull Request Requirements

Before submitting a PR, make sure:

1. **All tests pass**. Run `make test` and verify everything is green.
2. **No clippy warnings**. Run `make lint` to confirm.
3. **Conventional commits**. Use the format `<type>: <description>` for commit messages. Types include: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf`, `ci`.
4. **Tests included**. New functionality should come with tests. Aim for 80%+ coverage.
5. **One concern per PR**. Keep pull requests focused on a single change or feature.

## Reporting Issues

Use GitHub Issues for bug reports and feature requests. For security vulnerabilities, see [.github/SECURITY.md](.github/SECURITY.md).
