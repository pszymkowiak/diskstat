# CLAUDE.md — diskstat

## Project

**diskstat** — Fast TUI disk usage analyzer (WinDirStat/ncdu alternative) built in Rust.

## Commands

```bash
cargo build                    # Dev build
cargo build --release          # Release build
cargo fmt --all                # Format
cargo clippy --all-targets     # Lint
cargo test                     # Tests
cargo install --path .         # Install locally
```

## Pre-commit gate

```bash
cargo fmt --all && cargo clippy --all-targets && cargo test
```

## Release workflow

Push `feat:` or `fix:` commits to master → release-please creates a PR → merge it → builds + publishes 4 binaries (macOS ARM/Intel, Linux x86/ARM).

## Architecture

- `src/main.rs` — CLI (clap), event loop
- `src/app.rs` — App state, navigation, search
- `src/types.rs` — FileTree (arena-based), FileEntry
- `src/i18n.rs` — EN/FR auto-detected from LANG
- `src/utils.rs` — Shared utilities (format_size, truncate_str, format_age)
- `src/scanner/walk.rs` — Parallel scanner (rayon + macOS getattrlistbulk)
- `src/scanner/cache.rs` — SQLite directory cache
- `src/scanner/tree_cache.rs` — Binary tree serialization (DST2)
- `src/scanner/dupes.rs` — 3-pass duplicate detection (blake3)
- `src/ui/` — ratatui widgets (file_tree, treemap, dialogs, extensions, statusbar, menu)

## Rules

- Always sign commits: `git commit -s`
- Never use `unwrap()` in production
- All user-visible text must be in `i18n.rs` (EN + FR)
- Test utility functions in their module
- Zero clippy warnings tolerance

## Agents

- `improver` — Autonomous improvement + release agent (`.claude/agents/improver.md`)
