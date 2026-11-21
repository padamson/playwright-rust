#!/usr/bin/env bash
#
# Run the Verus proofs in playwright-rs (the glob translation in
# crates/playwright/src/protocol/glob.rs), installing the pinned Verus
# release and the Rust toolchain it pins first if either is missing.
#
# Usage: scripts/verus.sh [--hook] [extra cargo verus verify args]
#
# --hook is for the pre-commit hook: on a platform with no pinned build, or
# when the release or toolchain cannot be fetched (offline), it warns and
# passes, leaving enforcement to the `Verus proofs` CI job. A checksum
# mismatch fails either way.
#
# Verus ships its own vstd and pins its own toolchain (read from the
# release's version.json), so VERUS_VERSION here and the exact `vstd` pin in
# crates/playwright/Cargo.toml move together (`cargo verus new` prints the
# one a release expects). The digests are the release assets' SHA-256, as
# GitHub lists them.
set -euo pipefail
cd "$(dirname "$0")/.." || exit 1

hook=false
if [ "${1:-}" = "--hook" ]; then
  hook=true
  shift
fi

# Fail, or under --hook warn and pass.
unavailable() {
  echo "verus.sh: $1" >&2
  if $hook; then
    echo "verus.sh: skipping the proofs here; CI runs them" >&2
    exit 0
  fi
  exit 1
}

VERUS_VERSION=0.2026.11.02.8bf68ee

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)
    platform=arm64-macos
    digest=3086974d1afebf8b9f6fee2939b626486a12248c0dab528b9262ea652b863326 ;;
  Linux-x86_64)
    platform=x86-linux
    digest=f3a0119292df8855e4435da8130bee1873b0e6a60c306accf81a08ea687bba3c ;;
  *)
    unavailable "no pinned Verus build for $(uname -s)-$(uname -m)" ;;
esac

cache="${XDG_CACHE_HOME:-$HOME/.cache}/verus/$VERUS_VERSION"
bin="$cache/verus-$platform"

if [ ! -x "$bin/cargo-verus" ]; then
  zip="verus-$VERUS_VERSION-$platform.zip"
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/verus.XXXXXX")
  trap 'rm -rf "$tmp"' EXIT
  echo "verus.sh: installing Verus $VERUS_VERSION into $cache"
  curl --proto '=https' --tlsv1.2 -sSfL --retry 3 -o "$tmp/$zip" \
    "https://github.com/verus-lang/verus/releases/download/release/$VERUS_VERSION/$zip" ||
    unavailable "could not download $zip"
  echo "$digest  $tmp/$zip" | shasum -a 256 -c - >/dev/null
  mkdir -p "$cache"
  unzip -q -o "$tmp/$zip" -d "$cache"
  if [ ! -x "$bin/cargo-verus" ]; then
    echo "verus.sh: $zip did not unpack to $bin" >&2
    exit 1
  fi
fi

# "toolchain": "1.99.1-aarch64-apple-darwin (...)" -> 1.99.1
toolchain=$(sed -n 's/.*"toolchain": "\([0-9.]*\)-.*/\1/p' "$bin/version.json")
[ -n "$toolchain" ] || { echo "verus.sh: no toolchain in $bin/version.json" >&2; exit 1; }

if ! rustup run "$toolchain" rustc -V >/dev/null 2>&1; then
  rustup toolchain install "$toolchain" --profile minimal ||
    unavailable "could not install the Rust $toolchain toolchain"
fi

PATH="$bin:$PATH" PLAYWRIGHT_SKIP_DRIVER_DOWNLOAD=1 \
  cargo verus verify -p playwright-rs --no-default-features --features verus,ring "$@"
