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
  build.rs              assembles the pinned driver (npm + Node, ADR 0006)
crates/playwright-rs-macros/  locator!() proc macro (published separately)
crates/playwright-rs-trace/   trace-zip parser (published separately)
crates/xtask/           repo tasks (verify-driver-version, verify-changelog-links,
                        site snippets); publish = false
crates/site/            playwright-rust.dev landing page (Leptos/WASM, built by Trunk)
crates/site-e2e/        dogfoods the bindings against that site; also the deploy gate
supply-chain/           cargo-vet audit config (see skill)
docs/                   roadmap, ADRs, implementation plans, technical notes
docs/agent/             agent-integration guidance for downstream users
skills/                 the skill we ship to downstream users
.claude/skills/         contributor-only skills, plus a symlink to the above
```

**`crates/site` and `crates/site-e2e` are excluded from the workspace**
(wasm target; features that would conflict with the root build). Two
consequences worth knowing before they cost you time:

- **Workspace-wide commands miss them.** `cargo fmt`/`clippy --workspace`
  and `cargo nextest run --workspace` never reach these crates; only the
  per-manifest steps in [`pages.yml`](.github/workflows/pages.yml) do. After
  touching either crate, run those four commands (`--manifest-path
  crates/site{,-e2e}/Cargo.toml`, plus `--target wasm32-unknown-unknown`
  for `site`) rather than trusting a green workspace run.
- **They carry their own `Cargo.lock`.** `site-e2e` depends on
  `playwright-rs` by path, so a workspace dependency change silently
  staleness its lockfile, which then refreshes the next time anyone runs a
  cargo command there — surfacing as unrelated-looking dirt in `git status`.
  A pre-commit hook (`scripts/check-external-lockfiles.sh`) now flags it on
  the commits that cause it, with the refresh command. Nothing breaks
  meanwhile; no CI job uses `--locked` against these crates.

  **`crates/playwright/fuzz` has the same shape** (its own excluded
  workspace, its own lockfile, a path dep on `playwright-rs`) and is covered
  by the same hook. It drifts worse than the site crates because nothing
  routine touches it: it sat on `playwright-rs` 0.13.0 until 0.15.1 before
  anyone noticed.

## Skills (procedural reference)

Two audiences, kept apart by directory. `skills/` is what this repo
ships to the world; `.claude/skills/` is contributor-only. Claude Code
auto-discovers only the latter, so the shipped skill is symlinked into it
(`.claude/skills/playwright-rs-usage -> ../../skills/playwright-rs-usage`)
and there is still one source of truth. On a Windows checkout without
symlink support that link lands as a stray text file and the skill simply
does not auto-load in-repo, which costs nothing else.

**The directory split alone does not keep the contributor skills
unpublished.** It gates the Claude Code plugin, whose default discovery
reads `skills/` only, but the Agent Skills CLI scans `.claude/skills/`
too and happily offered all four to downstream users. Each
contributor-only skill therefore carries `metadata.internal: true`, which
hides it from `npx skills add` unless the caller sets
`INSTALL_INTERNAL_SKILLS=1`. **Any new skill added under
`.claude/skills/` needs that line**, or it ships to everyone who installs
from this repo. Note the value must be the boolean `true`, not the string
`"true"`: the CLI ignores the quoted form, even though the Agent Skills
spec describes `metadata` as string-to-string. Verify with
`npx skills add <path-to-this-repo> --list`, which should report exactly
one skill.

Load these when the task touches their domain:

- **supply-chain** — `cargo audit` / `cargo deny` / `cargo vet`
  workflow. Read before bumping our own version, before resolving a
  dependabot PR's vet failures, or when a `RUSTSEC-*` advisory drops.
- **doctest-conventions** — `no_run` doctests with hidden scaffolding;
  compile-checked everywhere. Read before authoring or modifying
  rustdoc examples.
- **release-process** — end-to-end release runbook including the
  push-commit-then-wait-for-CI-then-tag pattern. Read before driving
  a release manually.
- **playwright-rs-usage** — procedural reference for using
  playwright-rs as a downstream Rust dependency (object model,
  `locator!()` macro, builder pattern, auto-wait semantics, trace
  capture). This is the shipped one, canonical at
  `skills/playwright-rs-usage/`. Loaded automatically in sessions
  running in this repo, via the symlink.

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
6. **`skills/playwright-rs-usage/SKILL.md`** — the single agent-facing
   artifact distributed to downstream Rust projects. Installed with
   `npx skills add padamson/playwright-rust` or as a Claude Code plugin.
   `docs/agent/CLAUDE_SNIPPET.md` is now only a pointer to it: it used
   to be a hand-synced copy-paste duplicate, which is exactly the shape
   that goes stale and then teaches the old API. There is no second
   copy to keep in sync any more, so don't reintroduce one.

   Two build gates hold the skill to the code, and they check different
   things. `cargo xtask verify-agent-docs` compiles its `rust,no_run`
   blocks against the real crate, catching code that stopped working;
   it then asserts the skill *names* every API-gating cargo feature and
   browser engine, catching a capability that shipped undocumented.
   Neither checks that the surrounding prose is accurate. That residue
   is real, and the skill says so itself.

   The same skill is also installable as a Claude Code plugin:
   [`.claude-plugin/marketplace.json`](.claude-plugin/marketplace.json)
   makes this repo a marketplace. Default discovery picks up `skills/`
   and nothing else, so the contributor-facing skills in
   `.claude/skills/` stay in-repo without the manifest naming paths.
   **Editing the
   skill means bumping `version` in
   [`.claude-plugin/plugin.json`](.claude-plugin/plugin.json)** — that
   version is the update gate, and until it moves `/plugin update`
   tells installed consumers they are already current. A pre-commit
   hook (`scripts/check-plugin-version-bumped.sh`) enforces it.
   Validate manifest changes with `claude plugin validate .`.

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
  `common::setup_context()` helpers. To wait for an event/state change,
  use `common::poll_until(timeout, cond)` — never a fixed
  `tokio::time::sleep` before an assertion (flakes on loaded CI)
- **Doctests** — see the **doctest-conventions** skill
- **CI** runs Linux, macOS, Windows with Chromium + Firefox + WebKit

## Development Commands

```bash
# Tests (cargo-nextest required: cargo install cargo-nextest)
cargo nextest run                           # all tests
cargo nextest run -p playwright-rs --lib    # unit tests only (~2s, no browsers)
cargo nextest run -p playwright-rs -E 'test(locator)'

# Doctests (nextest does not run these; no_run = compile-only by design)
cargo test --doc --workspace                # pre-commit and CI run exactly this

# Examples
cargo run --package playwright-rs --example basic

# Quality
cargo fmt -- --check
cargo clippy --workspace --all-targets -- -D warnings

# Local CI rehearsal
prek run --all-files
```

## Claude Code sandbox

The sandbox is on (user settings) and `.claude/settings.json` (tracked)
carries this repo's additions: `git`, `gh`, `cargo nextest run`,
`cargo run`, `scripts/mutants.sh`, and `prek` run outside it because they
need credentials or the browser's Mach ports the sandbox hides; loopback
is allowed for the test servers; and the `configd` Mach lookup lets
`cargo vet`/`audit`/`deny` run sandboxed. Keep that file to the sandbox
block. Personal permission allowlists go in the untracked
`.claude/settings.local.json`. A session here never writes into a sibling
checkout. Hooks are installed once per clone by hand, since `.git/hooks`
is outside the sandbox's write set:

```bash
cargo install prek
prek install --overwrite   # replaces any legacy pre-commit hook
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
Saturday cron and on demand via `workflow_dispatch` — kept on a cadence
so test-quality drift across files outside the recent diff still gets
caught. Release tags are **not** a trigger: `security.yml` filters
`on.push` to branches, which excludes tag pushes entirely. Dispatch a
run manually if you want full coverage before cutting a release.

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
- Driver protocol schema — the wire contract this crate implements, and
  the authoritative thing to diff when bumping the driver. It is **not
  in the driver we assemble**: the `playwright-core` npm package ships
  no schema (only the retired CDN zips did), so fetch it from the source
  repo at the tag you care about:
  `https://raw.githubusercontent.com/microsoft/playwright/v<VERSION>/packages/protocol/spec/<file>.yml`
  Upstream split the former single `protocol.yml` into a
  [`packages/protocol/spec/`](https://github.com/microsoft/playwright/tree/main/packages/protocol/spec)
  directory (~19 files: `frame.yml`, `page.yml`, `browserContext.yml`,
  `core.yml`, ...), so a driver-bump review means diffing the directory
  across both tags, not one file.
