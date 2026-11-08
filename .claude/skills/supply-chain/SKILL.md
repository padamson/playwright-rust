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
`cargo vet`. Who read the diff decides how each one is settled. An audit
is recorded under the certifying developer's name and is imported by
anyone who trusts this repository's `audits.toml`, so it is their
attestation to every importer: record one only for a diff that developer
read. A review done only by an agent session is not that, however
thorough; it goes in as a plain exemption, and the commit message says
the review was a session's. An exemption stays local to this repo, so no
importer relies on it. A later read can turn the exemption into an
audit.

The same export works against you when an audit goes away. Removing or
narrowing one (replacing it with an exemption, or lowering its criteria)
takes that coverage away from every importer, and their `cargo vet` goes
red at its next run with nothing failing here. Audits, wildcard audits,
and trusts export; exemptions and policies don't. Before such a change
lands, find the importers with
`gh search code "playwright-rust/main/supply-chain/audits.toml"`, check
whether each one's tree has the crate, and tell its maintainers. The
commit message names each audit removed or narrowed.

### Clearing a vet failure, step by step

Run `cargo vet` without `--locked` first. With `--locked` it cannot
refresh `imports.lock`, so it misses the publisher trust and imported
audits for the new versions and reports crates CI does not. Then settle
each crate it lists, trying these in order:

- **Trust**, when vet says an imported audit set already trusts the
  crate's publisher (`cargo vet trust <crate> <publisher>`). This is a
  decision about the publisher, not a code review, and the cheapest
  honest answer when it applies.
- **Exemption**, when the certifying developer has not read the diff.
  Two kinds:
  - A plain one, `cargo vet add-exemption <crate> <version> --criteria
    <criteria>`, no notes, for a crate a session reviewed and the
    developer did not; the commit message says so.
  - One with notes, when there is a reason no review applies. The common
    case is a crate that sits in `Cargo.lock` but is never compiled:
    Cargo records an optional dependency even when no feature turns it
    on, and vet checks the lockfile. Prove it with
    `cargo tree --workspace --all-features -i <crate> -e all --target all`
    ("nothing to print" means no build reaches it), then add the
    exemption at `safe-to-run` with `--notes "..."` naming the dependency
    that drags it in.
- **Delta audit**, only for a diff the certifying developer read. Hand it
  over first (below); then record it with
  `cargo vet certify <crate> <old> <new> --criteria <criteria>
  --accept-all --notes "..."`. Leave `--who` off: vet then records the
  auditor from the certifying developer's `git config user.name` and
  `user.email`. The notes say what changed and what was checked, in the
  style of the existing `[[audits.*]]` entries.

### Where the fix lands: `main`, never the Dependabot branch

A commit on Dependabot's branch is lost the moment the PR is recreated:
Dependabot stops rebasing a PR someone else has pushed to, and
`@dependabot recreate` force-pushes over it. So the fix is its own commit
on `main`, and the PR picks it up by rebase:

1. On `main`, bring in the PR's dependency changes without committing
   them: `git fetch origin pull/<N>/head`, then
   `git diff HEAD...FETCH_HEAD -- '*Cargo.lock' '*Cargo.toml' | git apply`.
   Three dots take the PR's own changes, from where it branched, so
   anything `main` changed since stays. If the apply fails, the PR is
   behind `main` where it matters: comment `@dependabot rebase` first.
2. Run `cargo vet` and settle each crate as above. Every entry goes in
   *alongside* the old version's, never replacing it.
3. Put `main`'s files back,
   `git checkout HEAD -- '*Cargo.lock' '*Cargo.toml'`, and run `cargo vet`
   again: it must pass on `main`'s own graph.
   Entries for versions `main` does not have yet are fine there; vet only
   warns that they could be pruned.
4. Commit only `supply-chain/`, push, and comment `@dependabot rebase` on
   the PR. The rebase is Dependabot's own push, so its checks re-run with
   the new store and `dependabot-auto-merge.yml` merges it once they pass.
5. After the merge, `cargo vet prune` as its own change. It drops the old
   versions' entries, and also exemptions that imported audits now cover,
   which reaches past the crates the PR touched, so it never rides the fix.

### Handing the audits over for review

An audit is recorded under the git identity of the developer who
certifies it, so it is their attestation, and it is not recorded until
they have read the diff; a session's reading, summarized however well,
does not stand in for theirs. Make the review something they can copy
and paste, item by item:

- **Each audit**: the diff command, `cargo vet diff <crate> <old> <new>`
  (Sourcegraph in a browser; `--mode local` for the terminal), or a
  `.diff` file per crate from
  `diff -ru ~/.cargo/registry/src/<index>/<crate>-<old> .../<crate>-<new>`.
  That is the published source vet audits, which can differ from the git
  tag. Name the hunks where the judgment sits, say which parts are tests,
  and say how deep your own review went (every hunk read, or a targeted
  scan of the manifest and of `unsafe`, fs, net, process and env use).
- **Each trust**: say it is a policy call with nothing to read, and name
  the crates the same publisher is already trusted for.
- **Each exemption**: the `cargo tree` command above, or whatever proves
  its stated reason. A plain exemption for a session-only review has
  nothing to prove; name it as one, so the developer can read the diff
  and certify instead if they choose.

## Security advisories (`cargo audit` failures)

A new `RUSTSEC-YYYY-NNNN` advisory against a transitive dependency
warrants a patch release even if functional behavior is unchanged. The
typical fix is `cargo update -p <vulnerable-crate>` to a patched
version, plus a CHANGELOG `### Security` entry referencing the
advisory.

Advisories usually arrive as a `security-advisory` issue before they turn
a gate red. `advisory-monitor.yml` runs `scripts/check-advisories.sh`
daily, which runs the advisory half of the `cargo deny` and `cargo audit`
gates against the same trees, and keeps one issue open while either finds
something. The issue body has the triage steps, including which ignore
list each gate reads.
