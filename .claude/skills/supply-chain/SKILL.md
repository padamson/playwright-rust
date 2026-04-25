---
name: supply-chain
description: Procedure for keeping playwright-rust's cargo audit / cargo deny / cargo vet checks green when bumping the project's own version, when external crates change, and when a security advisory drops.
---

# Supply Chain & Release Hygiene

The project uses three complementary supply-chain tools:

- **`cargo audit`** — vulnerability advisories from RustSec
- **`cargo deny`** — license, duplicate, and source policy
- **`cargo vet`** — explicit audit chain for every dependency

All three run in CI on every push and every dependabot PR. Treat their
output as load-bearing — a real `cargo audit` failure is a security
advisory and warrants a patch release; a `cargo vet` failure usually
means an audit chain needs re-stitching after a version change.

## When bumping our own version

`supply-chain/imports.lock` is **generated, never hand-edited**. The
header comment says `# cargo-vet imports lock`. The
`[[unpublished.playwright-rs]]` entries handle the chicken-and-egg
window between bumping `Cargo.toml` and publishing to crates.io: an
entry like `version = "0.12.1" audited_as = "0.12.0"` tells vet "treat
the in-tree version as audited at the prior released version's level."

Proper sequence when bumping `Cargo.toml`:

1. Bump `version = "X.Y.Z"` in workspace `Cargo.toml`
2. Run `cargo vet regenerate unpublished` — automatically removes
   entries for now-published versions and adds a new `[[unpublished]]`
   entry chained to the prior version
3. If `cargo vet` still fails (the chain may break when prior versions
   get published and lose their `[[unpublished]]` placeholder), bump
   the `[[exemptions.playwright-rs]] version = "..."` line in
   `supply-chain/config.toml` to the latest published version. The
   exemption is the anchor that the unpublished entries chain to.
4. Verify `cargo vet`, `cargo audit`, `cargo deny check` all pass
5. Commit `Cargo.toml`, `Cargo.lock`, `supply-chain/imports.lock`, and
   any `supply-chain/config.toml` exemption bump together

## When external dependencies update (dependabot PRs)

Dependabot PRs that bump external crates often surface "missing
[safe-to-deploy]" or "missing [safe-to-run]" failures from
`cargo vet`. Resolve by either:

- Running `cargo vet diff <crate> <old> <new>` and `cargo vet certify`
  to record an explicit audit (preferred for small, reviewable diffs),
  **or**
- Adding an exemption in `supply-chain/config.toml` (acceptable for
  well-known crates with negligible delta — match the existing
  exemption style).

## Security advisories (`cargo audit` failures)

A new `RUSTSEC-YYYY-NNNN` advisory against a transitive dependency
warrants a patch release even if functional behavior is unchanged. The
typical fix is `cargo update -p <vulnerable-crate>` to a patched
version, plus a CHANGELOG `### Security` entry referencing the
advisory.
