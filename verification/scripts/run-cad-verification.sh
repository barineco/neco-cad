#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
request=${1:-$root/verification/vectors/requests/cad-request-batch.json}
artifacts=$root/verification/artifacts
mkdir -p "$artifacts/rust" "$artifacts/lean"
batch=$artifacts/rust/cad-observation-batch.json
cargo run --quiet --manifest-path "$root/verification/rust/Cargo.toml" -p cad-observation-producer -- generate --request "$request" > "$batch"
(cd "$root/verification/lean" && lake build)
checker=$root/verification/lean/.lake/build/bin/neco-cad-checker
checker_result=$artifacts/lean/cad-checker-result.json
"$checker" check "$batch" > "$checker_result"
cargo run --manifest-path "$root/verification/observer/Cargo.toml" --quiet -- --batch "$batch" --checker-result "$checker_result"
