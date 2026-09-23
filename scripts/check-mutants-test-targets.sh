#!/usr/bin/env bash
#
# The mutation kill set in .cargo/mutants.toml names test binaries one by
# one, because listing them by target is what keeps cargo from relinking
# the browser suites for every mutant. The cost of a list is that a new
# browser-free test binary is silently left out: tests/properties.rs sat
# outside it for months, and every mutant its cases would have killed was
# reported as if the property tests did not exist.
#
# This checks that every crates/playwright/tests/*.rs is either in the kill
# set or in the short list of suites that need a browser.
set -uo pipefail

config=.cargo/mutants.toml
# Suites that launch a browser or a pty, and so cannot run per mutant.
browser_suites=(integration sigint_termios)

status=0
for file in crates/playwright/tests/*.rs; do
  name="$(basename "$file" .rs)"
  for suite in "${browser_suites[@]}"; do
    [[ "$name" == "$suite" ]] && continue 2
  done
  if ! grep -Eq "^\s*\"--test\",\s*\"$name\"" "$config"; then
    echo "test binary '$name' is not in the mutation kill set."
    echo "  add '\"--test\", \"$name\",' to additional_cargo_test_args in $config,"
    echo "  or to browser_suites in $0 if it launches a browser."
    status=1
  fi
done
exit $status
