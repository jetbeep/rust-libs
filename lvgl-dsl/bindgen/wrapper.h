#ifndef JETBEEP_LVGL_WRAPPER_H
#define JETBEEP_LVGL_WRAPPER_H

/* Make sure all CONFIG_* defines are available (normally done via -imacros in Zephyr build) */
#include <zephyr/autoconf.h>

/*
 * KERNEL causes syscalls to not be implemented properly in bindgen context.
 * Also, errno_private.h needs errno to be defined.
 */
#undef KERNEL
extern int errno;

/* Gcc defines __SOFT_FP__ when the target uses software floating point */
#if defined(CONFIG_CPU_CORTEX_M)
#if !defined(CONFIG_FP_HARDABI) && !defined(__SOFTFP__)
#define __SOFTFP__
#endif
#endif

/*
 * Some Zephyr/LVGL build definitions can provide LV_CONF_PATH in a form that
 * clang/bindgen cannot use in '#include LV_CONF_PATH'. Force a stable include.
 */
#ifdef LV_CONF_PATH
#undef LV_CONF_PATH
#endif
#define LV_CONF_PATH "lv_conf.h"

#include <lvgl.h>

/*
 * Guard: the desktop-simulator and mock layers in src/c_bindings.rs
 * hard-code `lv_anim_t` as a 256-byte opaque buffer.  If the real struct
 * grows beyond that on any target/configuration, LVGL will write past the
 * buffer during lv_anim_init/setters, causing memory corruption.
 *
 * This fires at Zephyr bindgen / compile time.  If it triggers, update
 * the `_data: [u8; 256]` arrays in src/c_bindings.rs (both desktop and
 * mock sections) to match the new size, and raise this limit accordingly.
 */
_Static_assert(sizeof(lv_anim_t) <= 256,
    "lv_anim_t exceeds 256-byte Rust buffer — update c_bindings.rs lv_anim_t size in both desktop and mock sections");

#endif /* JETBEEP_LVGL_WRAPPER_H */
