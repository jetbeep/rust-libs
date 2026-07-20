#!/usr/bin/env bash
# Simulate the production Zephyr bindgen path against pinned LVGL v9.3 headers.
#
# This does not replace the real Zephyr SDK build. It supplies a small Zephyr
# header/config stub so build.rs runs bindgen, then `cargo check --lib` compiles
# the crate without cfg(no_zephyr). Missing allowlist entries or wrong generated
# enum names fail the check before changes can reach device CI.

set -euo pipefail

cd "$(dirname "$0")/.."

LVGL_REF="${LVGL_REF:-v9.3.0}"
WORK_DIR=".ci/lvgl-93-bindgen"
LVGL_DIR="${LVGL_93_DIR:-$WORK_DIR/lvgl}"
STUB_DIR="$WORK_DIR/stub"
TARGET_DIR="$WORK_DIR/target"

ensure_lvgl_tree() {
  if [ -n "${LVGL_93_DIR:-}" ]; then
    return
  fi

  local parent_lvgl="../../../third_party/lvgl"
  rm -rf "$LVGL_DIR"
  mkdir -p "$LVGL_DIR"

  if git -C "$parent_lvgl" rev-parse --verify --quiet "$LVGL_REF^{commit}" >/dev/null 2>&1; then
    git -C "$parent_lvgl" archive "$LVGL_REF" | tar -x -C "$LVGL_DIR"
  else
    git clone --quiet --depth 1 --branch "$LVGL_REF" https://github.com/lvgl/lvgl.git "$LVGL_DIR"
  fi
}

prepare_stubs() {
  rm -rf "$STUB_DIR"
  mkdir -p \
    "$STUB_DIR/generated/include/zephyr" \
    "$STUB_DIR/include" \
    "$STUB_DIR/zephyr-base/lib/libc/minimal/include" \
    "$STUB_DIR/zephyr-base/modules/lvgl/include"

  cat >"$STUB_DIR/generated/include/zephyr/autoconf.h" <<'EOF'
#define CONFIG_LVGL 1
#define CONFIG_LV_Z_MEM_POOL_HEAP_LIB_C 1
EOF

  python3 - "$LVGL_DIR/lv_conf_template.h" "$STUB_DIR/include/lv_conf.h" <<'PY'
from pathlib import Path
import re
import sys

src = Path(sys.argv[1]).read_text()
src = src.replace("#if 0 /* Set this to \"1\" to enable content */", "#if 1 /* enabled by check_lvgl_93_bindgen.sh */", 1)
for name in [
    "LV_USE_QRCODE",
    "LV_FONT_MONTSERRAT_20",
    "LV_FONT_MONTSERRAT_24",
    "LV_FONT_MONTSERRAT_30",
    "LV_FONT_MONTSERRAT_32",
    "LV_FONT_MONTSERRAT_40",
    "LV_FONT_MONTSERRAT_48",
]:
    src = re.sub(rf"^#define {name}\s+0\b", f"#define {name} 1", src, flags=re.MULTILINE)
Path(sys.argv[2]).write_text(src)
PY

  cat >"$STUB_DIR/zephyr-base/modules/lvgl/include/lvgl.h" <<EOF
#include "$(pwd)/$LVGL_DIR/lvgl.h"
EOF
}

run_check() {
  rm -rf "$TARGET_DIR"
  ZEPHYR_BASE="$(pwd)/$STUB_DIR/zephyr-base" \
  INCLUDE_DIRS="$(pwd)/$STUB_DIR/generated/include;$(pwd)/$STUB_DIR/include;$(pwd)/$LVGL_DIR;$(pwd)/$LVGL_DIR/src" \
  INCLUDE_DEFINES="LV_CONF_INCLUDE_SIMPLE" \
  BINARY_DIR_INCLUDE_GENERATED="$(pwd)/$STUB_DIR/generated/include" \
  CARGO_TARGET_DIR="$TARGET_DIR" \
    cargo check --lib --quiet
}

assert_generated_names() {
  local bindings
  bindings=$(find "$TARGET_DIR" -path '*/out/bindings.rs' -print | head -n 1)
  if [ -z "$bindings" ]; then
    echo "No generated bindings.rs found under $TARGET_DIR" >&2
    return 1
  fi

  local required=(
    LV_LABEL_LONG_MODE_WRAP
    LV_LABEL_LONG_MODE_DOTS
    LV_LABEL_LONG_MODE_SCROLL
    LV_LABEL_LONG_MODE_SCROLL_CIRCULAR
    LV_LABEL_LONG_MODE_CLIP
    LV_PART_MAIN
    LV_PART_INDICATOR
    LV_PART_KNOB
    LV_EVENT_CLICKED
    LV_EVENT_VALUE_CHANGED
    LV_EVENT_SCROLL_END
    LV_RESULT_OK
    lv_font_montserrat_20
    lv_font_montserrat_24
    lv_font_montserrat_30
    lv_font_montserrat_32
    lv_font_montserrat_40
    lv_font_montserrat_48
  )

  for name in "${required[@]}"; do
    if ! grep -Eq "(pub const|pub static).*\\b${name}\\b" "$bindings"; then
      echo "Generated bindings are missing $name" >&2
      return 1
    fi
  done

  if grep -Eq "LV_LABEL_LONG_(WRAP|DOT|SCROLL|SCROLL_CIRC|CLIP)\\b" "$bindings"; then
    echo "Generated bindings unexpectedly contain deprecated LV_LABEL_LONG_* names" >&2
    return 1
  fi
}

ensure_lvgl_tree
prepare_stubs
run_check
assert_generated_names

echo "[lvgl-93-bindgen] OK — bindgen + Zephyr-cfg cargo check succeeded against LVGL $LVGL_REF."
