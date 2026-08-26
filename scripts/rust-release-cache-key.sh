#!/usr/bin/env bash
set -euo pipefail

rustc_version="$(rustc -Vv)"
toolchain_hash="$({
    printf '%s' "$rustc_version" | shasum -a 256 | awk '{print $1}'
})"
printf 'toolchain=%s\n' "$toolchain_hash" >> "${GITHUB_OUTPUT:-/dev/stdout}"
