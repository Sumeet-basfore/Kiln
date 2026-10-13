#!/usr/bin/env bash
set -euo pipefail

# scripts/check-deps.sh
# Verifies that dependencies between crates point downward only,
# strictly enforcing the architecture dependency rules.
# Also verifies that no library crate lists external dependencies for now.
# Note: kiln-benches is developer tooling, not a product crate, and is ignored.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Checking crate dependency directions and external dependencies..."

# ==============================================================================
# ALLOWED INTERNAL DEPENDENCIES TABLE
# ==============================================================================
# Crate           | Allowed internal dependencies
# ----------------+-------------------------------------------------------------
# kiln-text       | (none)
# kiln-core       | kiln-text
# kiln-syntax     | kiln-text
# kiln-config     | (none)
# kiln-services   | kiln-core kiln-syntax kiln-config
# kiln-input      | kiln-core kiln-config
# kiln-ui         | kiln-core kiln-config (kiln-syntax disallowed until T12)
# kiln-bin        | kiln-text kiln-core kiln-syntax kiln-config kiln-services kiln-input kiln-ui
# ==============================================================================

declare -A ALLOWED_INTERNAL_DEPS=(
    ["kiln-text"]=""
    ["kiln-core"]="kiln-text"
    ["kiln-syntax"]="kiln-text"
    ["kiln-config"]=""
    ["kiln-services"]="kiln-core kiln-syntax kiln-config"
    ["kiln-input"]="kiln-core kiln-config"
    ["kiln-ui"]="kiln-core kiln-config"
    ["kiln-bin"]="kiln-text kiln-core kiln-syntax kiln-config kiln-services kiln-input kiln-ui"
)

LIBRARY_CRATES=(
    "kiln-text"
    "kiln-core"
    "kiln-syntax"
    "kiln-config"
    "kiln-services"
    "kiln-input"
    "kiln-ui"
)

ALL_PRODUCT_CRATES=(
    "${LIBRARY_CRATES[@]}"
    "kiln-bin"
)

ALL_WORKSPACE_CRATES=(
    "${ALL_PRODUCT_CRATES[@]}"
    "kiln-benches"
)

FAILED=0

# Extract dependency keys from [dependencies], [dev-dependencies], and [build-dependencies]
extract_deps() {
    local manifest="$1"
    awk '
    BEGIN { in_dep = 0 }
    /^\[(dependencies|dev-dependencies|build-dependencies)\]/ { in_dep = 1; next }
    /^\[.*\]/ { in_dep = 0; next }
    in_dep && /^[[:alnum:]_-]+[[:space:]]*=/ {
        split($0, parts, "=");
        gsub(/[[:space:]]/, "", parts[1]);
        print parts[1];
    }
    ' "$manifest"
}

for crate in "${ALL_PRODUCT_CRATES[@]}"; do
    manifest="$ROOT_DIR/crates/$crate/Cargo.toml"
    if [ ! -f "$manifest" ]; then
        echo "FAIL: Manifest not found: $manifest"
        FAILED=1
        continue
    fi

    allowed="${ALLOWED_INTERNAL_DEPS[$crate]}"
    deps=$(extract_deps "$manifest" || true)

    for dep in $deps; do
        is_workspace_crate=0
        for wc in "${ALL_WORKSPACE_CRATES[@]}"; do
            if [ "$dep" = "$wc" ]; then
                is_workspace_crate=1
                break
            fi
        done

        if [ "$is_workspace_crate" -eq 1 ]; then
            # Check internal dependency direction
            is_allowed=0
            for item in $allowed; do
                if [ "$item" = "$dep" ]; then
                    is_allowed=1
                    break
                fi
            done

            if [ "$is_allowed" -eq 0 ]; then
                echo "FAIL: Crate '$crate' has illegal internal dependency on '$dep'!"
                FAILED=1
            fi
        else
            # For now, library crates must not have external dependencies
            is_lib=0
            for lib in "${LIBRARY_CRATES[@]}"; do
                if [ "$crate" = "$lib" ]; then
                    is_lib=1
                    break
                fi
            done

            if [ "$is_lib" -eq 1 ]; then
                echo "FAIL: Library crate '$crate' has disallowed external dependency '$dep'!"
                FAILED=1
            fi
        fi
    done
done

if [ "$FAILED" -eq 1 ]; then
    echo "Dependency direction check FAILED."
    exit 1
else
    echo "Dependency direction check PASSED."
    exit 0
fi
