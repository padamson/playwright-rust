# Changelog index

This workspace ships **three independently-versioned crates**. Each
crate maintains its own CHANGELOG; release-notes generation reads the
matching file based on the tag prefix that triggered the release
(see [`.github/workflows/release.yml`](.github/workflows/release.yml)).

| Tag prefix    | Crate                                                                               | CHANGELOG                                                                          |
|---------------|-------------------------------------------------------------------------------------|------------------------------------------------------------------------------------|
| `v*`          | [`playwright-rs`](crates/playwright/)                                               | [`crates/playwright/CHANGELOG.md`](crates/playwright/CHANGELOG.md)                 |
| `macros-v*`   | [`playwright-rs-macros`](crates/playwright-rs-macros/)                              | [`crates/playwright-rs-macros/CHANGELOG.md`](crates/playwright-rs-macros/CHANGELOG.md) |
| `trace-v*`    | [`playwright-rs-trace`](crates/playwright-rs-trace/)                                | [`crates/playwright-rs-trace/CHANGELOG.md`](crates/playwright-rs-trace/CHANGELOG.md)   |

The `xtask` workspace member is `publish = false` (build tooling, not
a published crate). Workspace-level conventions live in the project
[`README.md`](README.md) and [`CLAUDE.md`](CLAUDE.md).

## Why per-crate, not unified

Independent versioning means each crate cuts its own release on its own
schedule. Per-crate CHANGELOGs are the [Keep a
Changelog](https://keepachangelog.com/) convention for multi-crate Rust
workspaces (see tokio, clap, serde, rustls). Each file is what
crates.io renders on the matching crate's page; release-note generation
fishes the right section out automatically.

## Pre-1.0 caveat

All three crates are pre-1.0. APIs may change between minor versions.
Migration notes for each breaking change live in the originating
crate's CHANGELOG under `### Breaking changes`.
