#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
cargo test --manifest-path "$root/verification/observer/Cargo.toml"
