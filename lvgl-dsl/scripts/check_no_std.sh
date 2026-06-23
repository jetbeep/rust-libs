#!/usr/bin/env bash
# Static guard: ensure production-mode (#![no_std]) code paths never import std.
#
# Why this exists:
#   src/lib.rs is `#![cfg_attr(not(any(test, no_zephyr)), no_std)]` — it only
#   becomes a true no_std crate when built against the real Zephyr toolchain.
#   Mock + test paths legitimately use std (HashMap, Mutex, Rc, etc.) and are
#   correctly cfg-gated. This script verifies that no `std::` reference can
#   reach the production build (i.e. lives outside `#[cfg(test)]` and outside
#   the mock/desktop_sim cfg-gated regions in `c_bindings.rs`).
#
# Non-goals: this is not a full no_std cross-compile. For that you'd need a
# Zephyr toolchain or a hand-maintained LVGL bindings stub. Use this as a fast
# pre-commit/CI gate; rely on the real Zephyr build for ultimate verification.

set -euo pipefail

cd "$(dirname "$0")/.."

fail=0

# ---------------------------------------------------------------------------
# 1. searchbar/* must be 100% std-free in production (it has zero std-using
#    cfg-gated regions outside `#[cfg(test)]`).
# ---------------------------------------------------------------------------
echo "[no_std] auditing src/lvgl/searchbar/ for production-mode std usage..."

# All `std::` matches inside the searchbar tree, with file+line.
matches=$(grep -RIn --include='*.rs' -E '\b(use\s+std::|\bstd::[a-z])' \
  src/lvgl/searchbar 2>/dev/null || true)

if [ -n "$matches" ]; then
  # Filter out lines that are clearly inside `#[cfg(test)]` regions.
  # We do a structural check: for each match, walk backwards in the file to
  # the nearest preceding `#[cfg(...)]` or `#[cfg_attr(...)]` and verify it
  # contains `test`. This is a heuristic but is reliable for the current
  # codebase shape (test modules are top-level cfg-gated `mod tests`).
  while IFS= read -r line; do
    file=$(echo "$line" | cut -d: -f1)
    lineno=$(echo "$line" | cut -d: -f2)
    # Find the nearest preceding `#[cfg(test)]` or `mod tests` boundary.
    in_test=$(awk -v target="$lineno" '
      /^[[:space:]]*#\[cfg\(test\)\]/        { gate = "test"; next }
      /^[[:space:]]*#\[cfg\(any\(test/        { gate = "test"; next }
      /^[[:space:]]*#\[cfg\(/                { gate = "non-test"; next }
      NR == target                            { print gate; exit }
    ' "$file")
    if [ "$in_test" != "test" ]; then
      echo "  ✗ $file:$lineno (not under #[cfg(test)])" >&2
      fail=1
    fi
  done <<< "$matches"
fi

if [ "$fail" -eq 0 ]; then
  echo "  ✓ searchbar/ clean (only std under #[cfg(test)])"
fi

# ---------------------------------------------------------------------------
# 2. Forbidden patterns ANYWHERE in src/ outside cfg-gated regions:
#    format!, println!, eprintln!, dbg!, std::thread, std::sync::Mutex/Arc/RwLock,
#    std::collections::HashMap/HashSet — these are common no_std footguns.
#    We allow them inside src/c_bindings.rs (mock module is cfg-gated) and
#    inside #[cfg(test)] blocks.
# ---------------------------------------------------------------------------
echo "[no_std] auditing src/lvgl/ for forbidden std-only macros/types..."

forbidden_re='\b(format!|println!|eprintln!|dbg!|std::thread|std::sync::(Mutex|Arc|RwLock|Once|RwLockReadGuard|RwLockWriteGuard)|std::collections::(HashMap|HashSet|BTreeMap|BTreeSet))\b'
matches=$(grep -RIn --include='*.rs' -E "$forbidden_re" src/lvgl 2>/dev/null || true)

if [ -n "$matches" ]; then
  while IFS= read -r line; do
    file=$(echo "$line" | cut -d: -f1)
    lineno=$(echo "$line" | cut -d: -f2)
    in_test=$(awk -v target="$lineno" '
      /^[[:space:]]*#\[cfg\(test\)\]/        { gate = "test"; next }
      /^[[:space:]]*#\[cfg\(any\(test/        { gate = "test"; next }
      /^[[:space:]]*#\[cfg\(/                { gate = "non-test"; next }
      NR == target                            { print gate; exit }
    ' "$file")
    if [ "$in_test" != "test" ]; then
      echo "  ✗ $file:$lineno (not under #[cfg(test)])" >&2
      fail=1
    fi
  done <<< "$matches"
fi

if [ "$fail" -eq 0 ]; then
  echo "  ✓ src/lvgl/ clean of std-only macros/types in production paths"
fi

# ---------------------------------------------------------------------------
# 3. Verify the no_std cfg gate in lib.rs is intact.
# ---------------------------------------------------------------------------
echo "[no_std] verifying lib.rs no_std gate..."
if ! grep -q '^#!\[cfg_attr(not(any(test, no_zephyr)), no_std)\]' src/lib.rs; then
  echo "  ✗ src/lib.rs is missing the production no_std cfg gate" >&2
  fail=1
else
  echo "  ✓ src/lib.rs no_std gate present"
fi

if ! grep -q '^extern crate alloc;' src/lib.rs; then
  echo "  ✗ src/lib.rs is missing 'extern crate alloc;'" >&2
  fail=1
else
  echo "  ✓ extern crate alloc declared"
fi

# ---------------------------------------------------------------------------
# 4. Verify the lib still builds in dev mode (mock backend).
# ---------------------------------------------------------------------------
echo "[no_std] verifying dev-mode build (cargo check --lib)..."
if cargo check --lib --quiet 2>&1; then
  echo "  ✓ cargo check --lib passes"
else
  echo "  ✗ cargo check --lib failed" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo ""
  echo "[no_std] FAILED — fix the issues above before pushing."
  exit 1
fi

echo ""
echo "[no_std] OK — production-mode code paths are std-free."
