---
name: supply-chain
description: Procedure for keeping playwright-rust's cargo audit / cargo deny / cargo vet checks green when bumping the project's own version, when external crates change, and when a security advisory drops.
metadata:
  internal: true
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

Nothing to do in `supply-chain/`. Each published workspace crate has
`[policy.<crate>] audit-as-crates-io = false` in `supply-chain/config.toml`,
so vet treats it as our own code and never asks for an audit of it, at
any version. Bump `Cargo.toml`, let `Cargo.lock` follow, and run
`cargo vet` as usual.

The policy used to be `true`, which made vet treat the path crates as
third-party crates.io code. Every bump then needed a
`[[unpublished]]` entry in `imports.lock` chained to an exemption for the
last published version, and the exemption had to be bumped by hand when
the chain broke. If either of those reappears, the policy has been
flipped back.

`supply-chain/imports.lock` is **generated, never hand-edited**. Its
header comment says `# cargo-vet imports lock`.

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
