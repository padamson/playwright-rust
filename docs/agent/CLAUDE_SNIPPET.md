# Playwright-rs agent guidance

This used to be a copy-paste section for your project's `CLAUDE.md`, kept
in sync by hand with the skill it duplicated. Install the skill instead:

```bash
npx skills add padamson/playwright-rust
```

It works with Claude Code, Codex, Cursor, and any other
[compatible agent](https://agentskills.io/clients), and Claude Code can
take it as a plugin instead:

```bash
/plugin marketplace add padamson/playwright-rust
/plugin install playwright-rs@playwright-rust
```

Either route updates with one command. A pasted copy does not: it goes
stale the moment the crate changes and then actively teaches the old API,
which is why this file no longer carries the content. The skill is
[`skills/playwright-rs-usage/`](../../skills/playwright-rs-usage/), and a
build gate holds it to the crate's real API surface.
