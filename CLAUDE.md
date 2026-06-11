# CLAUDE.md

Guidance for Claude Code working in this repository.

## Project

**playwright-rust** — Rust language bindings for Microsoft Playwright,
following the same architecture as playwright-python / java / dotnet.
JSON-RPC to the official Playwright server (we don't reimplement
browser protocols). Goal: production-quality bindings, full Python API
parity (achieved in v0.12.0), then v1.0 after multi-month dogfooding.

See [WHY.md](WHY.md) for vision, [docs/roadmap.md](docs/roadmap.md) for
direction, and [docs/implementation-plans/v1.0-gap-analysis.md](docs/implementation-plans/v1.0-gap-analysis.md)
for current state.

## Repository Layout

```
crates/playwright/      single crate (consolidated from playwright-core in v0.7)
  src/api/              launch options, connect options
  src/protocol/         protocol objects (Page, Browser, Locator, ...)
  src/server/           connection, transport, channel, object factory
  src/assertions.rs     expect API (auto-retry assertions)
  src/error.rs          error types
  tests/integration/    integration tests
  examples/             usage examples
  fuzz/                 cargo-fuzz targets
drivers/                Playwright server binaries (gitignored)
supply-chain/           cargo-vet audit config (see skill)
docs/                   roadmap, ADRs, implementation plans, technical notes
docs/agent/             agent-integration guidance for downstream users
.claude/skills/         procedural reference (see below)
```

## Skills (procedural reference)

Load these (`.claude/skills/<name>/SKILL.md`) when the task touches
their domain:

- **supply-chain** — `cargo audit` / `cargo deny` / `cargo vet`
  workflow. Read before bumping our own version, before resolving a
  dependabot PR's vet failures, or when a `RUSTSEC-*` advisory drops.
- **doctest-conventions** — module-level doctests with `ignore`
  annotation. Read before authoring or modifying rustdoc examples.
- **release-process** — end-to-end release runbook including the
  push-commit-then-wait-for-CI-then-tag pattern. Read before driving
  a release manually.
- **playwright-rs-usage** — procedural reference for using
  playwright-rs as a downstream Rust dependency (object model,
  `locator!()` macro, builder pattern, auto-wait semantics, trace
  capture). Mirrors the artifact we ship for downstream copy (see
  "Agent-integration artifacts" below). Loaded automatically in
  sessions running in this repo.

## Documentation Hierarchy

Just-in-time philosophy — write the right thing in the right file:

1. **README.md** — landing page; vision, working example (current code
   only), what works now, installation. Keep < 250 lines. No future
   API previews.
2. **docs/roadmap.md** — strategic direction, milestone planning,
   high-level version overview. No slice details.
3. **docs/implementation-plans/vX.Y-*.md** — detailed work tracking
   for the version *currently in progress*; created just-in-time.
   Becomes a historical reference once the version ships.
4. **docs/adr/####-*.md** — architecture decisions with trade-off
   analysis. Use [docs/templates/TEMPLATE_ADR.md](docs/templates/TEMPLATE_ADR.md).
5. **Rustdoc** — every public API gets a summary, link to Playwright
   docs (`See: <https://playwright.dev/...>`), errors section, and any
   Rust-specific behavior notes. Examples go in module-level doctests
   per the doctest-conventions skill, not on individual functions.
6. **`docs/agent/`** — guidance distributed to downstream Rust
   projects that consume this crate from a Claude Code / agent
   workflow. `CLAUDE_SNIPPET.md` is the copy-paste version;
   `.claude/skills/playwright-rs-usage/SKILL.md` is the in-repo
   canonical that downstream users can also `cp -r` into their own
   `.claude/skills/`. Keep both in sync — the snippet is the
   short-form version of the skill.

## Working on Features

1. Always check Playwright's official API docs first (and
   playwright-python as the reference implementation).
2. Default to TDD: write the failing test, make it pass, refactor.
   For new APIs that's Red → Green → Refactor against the cross-browser
   integration suite + an API-compatibility check against
   playwright-python.
3. Match Playwright's API exactly across languages — same method
   names, same semantics. Diverge only for idiomatic Rust where
   compatibility allows (`Result<T>`, builders for option-heavy
   methods, async/await).

## API Conventions

- `Result<T>` consistently; one `Error` enum (`crate::error::Error`)
- Builder pattern for option-heavy methods (matches Playwright's
  `LaunchOptions`, `GotoOptions`, `ClickOptions` style)
- Locators auto-wait for elements; assertions auto-retry — see the
  expect API (`crate::assertions`)
- No unsafe code without a `// SAFETY:` justification
- No `unwrap()`/`expect()` on fallible paths reachable from public
  APIs — return an `Error` variant. Two sanctioned exceptions:
  `std::sync` lock acquisition (poisoning means another thread already
  panicked; propagating that panic is the policy) and invariants
  guaranteed by construction (comment why at the call site)

## Testing

- **Unit tests** — protocol serialization, connection management,
  server lifecycle (in `crates/playwright/src/`)
- **Integration tests** — end-to-end API exercising real browsers
  (`crates/playwright/tests/integration/`); use `common::setup()` /
  `common::setup_context()` helpers
- **Doctests** — see the **doctest-conventions** skill
- **CI** runs Linux, macOS, Windows with Chromium + Firefox + WebKit

## Development Commands

```bash
# Tests (cargo-nextest required: cargo install cargo-nextest)
cargo nextest run                           # all tests
cargo nextest run -p playwright-rs --lib    # unit tests only (~2s, no browsers)
cargo nextest run -p playwright-rs -E 'test(locator)'

# Doctests (nextest does not run these)
cargo test --doc                            # compile-only (pre-commit)
cargo test --doc --workspace -- --ignored   # full execution (CI)

# Examples
cargo run --package playwright-rs --example basic

# Quality
cargo fmt -- --check
cargo clippy --workspace --all-targets -- -D warnings

# Local CI rehearsal
pre-commit run --all-files
```

## Mutation testing

`scripts/mutants.sh` wraps `cargo mutants --in-diff`, scoping mutation
testing to just the lines a commit touched. A full-codebase run grows
linearly with codebase size and routinely takes hours; `--in-diff`
keeps the loop fast enough to use while the test is still warm.

```bash
./scripts/mutants.sh                 # diff HEAD~1..HEAD (default)
./scripts/mutants.sh main            # diff main..HEAD
./scripts/mutants.sh -- --jobs 4     # pass extra cargo-mutants args
```

CI runs the per-diff variant on every push and PR
(`mutation-testing-diff` in [`security.yml`](.github/workflows/security.yml)).
The full-codebase job (`mutation-testing`) runs on the weekly
Saturday cron, on release tag pushes, and on demand via
`workflow_dispatch` — kept on a cadence so test-quality drift across
files outside the recent diff still gets caught.

Scope is set by [`.cargo/mutants.toml`](.cargo/mutants.toml)
(`examine_globs` lists the files that get mutated at all; `exclude_re`
removes mutants that are only testable via integration tests).
`--in-diff` narrows from there.

Install once: `cargo install cargo-mutants`.

## Versioning

`0.x.y` while pre-1.0; API may evolve. `1.0.0` after stable parity is
proven through dogfooding (see roadmap). For release mechanics see the
**release-process** skill.

## Useful References

- Playwright docs: <https://playwright.dev/docs/api>
- playwright-python (reference impl): <https://github.com/microsoft/playwright-python>
- Playwright server source: <https://github.com/microsoft/playwright/tree/main/packages/playwright-core/src/server>
- Driver protocol schema: `drivers/playwright-*/package/protocol.yml`
