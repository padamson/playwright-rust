# Vendored Playwright protocol spec

The wire contract this crate implements, taken verbatim from
`packages/protocol/spec/` in the Playwright repository at the tag of the
driver `crates/playwright/build.rs` pins. `DRIVER_VERSION` records which
one.

It is vendored rather than fetched on demand so that **a driver bump shows
its wire-contract delta as a diff in the bump commit**. That matters
because the driver's validator copies the parameters it knows and drops
the rest: a renamed parameter is not rejected, it is ignored. Playwright
1.63 renamed every `tracingStart` capture parameter, and traces recorded
with the old spellings came out with no DOM to replay and no timeline,
with no error anywhere and every existing test still passing.

## Refreshing it

Part of bumping the driver, after `PLAYWRIGHT_VERSION` moves:

```bash
cargo xtask sync-protocol-spec     # fetches at the newly pinned tag
git diff protocol-spec             # this diff is the review
```

Read the diff for removed or renamed parameters on commands this crate
sends, then for added ones worth surfacing.

`cargo xtask sync-protocol-spec --check` runs offline in CI and
pre-commit. It fails when the vendored copy is not the pinned driver's,
and `MANIFEST` lets it also catch a file that was edited, lost, or added
by hand, so "vendored verbatim" is checked rather than assumed. The sync
fetches every file before writing any, so a failed refresh leaves the
previous copy intact instead of a half-written tree the stamp still
vouches for.

Nothing here is compiled or published; the crate reads the driver's
behavior, not these files.
