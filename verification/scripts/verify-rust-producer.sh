#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
if grep -q 'verification/rust' "$root/Cargo.toml"; then
  echo "product workspace must not list verification/rust" >&2
  exit 1
fi
cargo test --manifest-path "$root/verification/rust/Cargo.toml"
