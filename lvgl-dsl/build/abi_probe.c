/* ABI probe compiled by build.rs in desktop-sim mode (LVGL_INCLUDE_DIRS set).
 *
 * Compile-time cross-checks between the values/layouts hardcoded on the Rust
 * side (src/c_bindings.rs `lv_shared_consts`, `lv_anim_t` opaque size,
 * `StaticStyle` mirror in src/lvgl/static_style.rs) and the real LVGL headers
 * the simulator links against. If the vendored LVGL version drifts from the
 * pinned v9.3.0 ABI, the desktop build fails here instead of misbehaving at
 * runtime.
 */
#include <lvgl.h>
#include <stddef.h>

/* Event numbering (v9.3.0 src/misc/lv_event.h). */
_Static_assert(LV_EVENT_CLICKED == 10, "LV_EVENT_CLICKED drifted from v9.3 value 10");
_Static_assert(LV_EVENT_SCROLL_END == 14, "LV_EVENT_SCROLL_END drifted from v9.3 value 14");
_Static_assert(LV_EVENT_VALUE_CHANGED == 35, "LV_EVENT_VALUE_CHANGED drifted from v9.3 value 35");
_Static_assert(LV_EVENT_READY == 38, "LV_EVENT_READY drifted from v9.3 value 38");
_Static_assert(LV_EVENT_CANCEL == 39, "LV_EVENT_CANCEL drifted from v9.3 value 39");
_Static_assert(LV_EVENT_DELETE == 41, "LV_EVENT_DELETE drifted from v9.3 value 41");

/* Rust declares lv_anim_t as an opaque [u8; 256]. */
_Static_assert(sizeof(lv_anim_t) <= 256, "lv_anim_t grew past the 256-byte Rust mirror");

/* StaticStyle mirrors the LV_USE_ASSERT_STYLE == 0 layout of lv_style_t:
 * { void *values_and_props; uint32_t has_group; uint8_t prop_cnt; }. */
_Static_assert(LV_USE_ASSERT_STYLE == 0,
               "StaticStyle mirrors the LV_USE_ASSERT_STYLE==0 layout; lv_conf.h enabled the sentinel");
_Static_assert(offsetof(lv_style_t, values_and_props) == 0,
               "lv_style_t.values_and_props moved");
_Static_assert(offsetof(lv_style_t, has_group) == sizeof(void *),
               "lv_style_t.has_group moved");
_Static_assert(offsetof(lv_style_t, prop_cnt) == sizeof(void *) + sizeof(uint32_t),
               "lv_style_t.prop_cnt moved");

/* Anchor for the object file so the probe always compiles something. */
int jb_dsl_abi_probe_anchor(void);
int jb_dsl_abi_probe_anchor(void) { return 0; }
