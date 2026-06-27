#ifndef JETBEEP_CORE_BINDGEN_WRAPPER_H
#define JETBEEP_CORE_BINDGEN_WRAPPER_H

#include <zephyr/autoconf.h>

/* Keep bindgen parse context aligned with lvgl-dsl wrapper expectations. */
#undef KERNEL
extern int errno;

#include <zephyr/fs/fs.h>

#include <zephyr-libs/error/error.h>
#include <zephyr-libs/unix_time/unix_time.h>
#include <zephyr-libs/bus_common/poll_api/poll_api.h>
#include <application/i2c_jb_bus/i2c_jb_bus.h>

// C shims implemented in app/src/rust-bridge/fs.c
void fs_file_t_init_shim(struct fs_file_t *file);
void fs_dir_t_init_shim(struct fs_dir_t *dir);

#endif
