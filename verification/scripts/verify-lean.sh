#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
cd "$root/verification/lean"
lake build NecoCadVerify.Checker
lake build NecoCadVerify.Soundness
lake build
lake exe neco-cad-checker check ../vectors/known/cad-known-vectors.json
