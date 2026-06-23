use std::env;
use std::fs;
use std::path::Path;

use bindgen::Builder;

#[path = "build/lvgl_allowlists.rs"]
mod lvgl_allowlists;

fn main() {
    // Declare the custom cfgs so rustc does not warn about unknown cfgs.
    println!("cargo:rustc-check-cfg=cfg(no_zephyr)");
    println!("cargo:rustc-check-cfg=cfg(desktop_sim)");

    // Allow `cargo test` / rust-analyzer / dependency builds to work without
    // a Zephyr toolchain.  When ZEPHYR_BASE is absent we:
    //   1. Emit cfg(no_zephyr) so std is enabled in lib.rs.
    //   2a. If LVGL_INCLUDE_DIRS is set (desktop SDL simulator), emit cfg(desktop_sim)
    //       so c_bindings.rs links against the real LVGL symbols.
    //   2b. Otherwise write an empty bindings stub and activate the mock layer.
    if env::var("ZEPHYR_BASE").is_err() {
        println!("cargo:rustc-cfg=no_zephyr");
        let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set");
        fs::write(Path::new(&out_dir).join("bindings.rs"), b"")
            .expect("failed to write empty bindings stub");
        if env::var("LVGL_INCLUDE_DIRS").is_ok() {
            println!("cargo:rustc-cfg=desktop_sim");
        }
        return;
    }

    generate_bindings();
}

fn generate_bindings() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set");
    let zephyr_base = env::var("ZEPHYR_BASE").expect("ZEPHYR_BASE must be set");
    let target = env::var("TARGET").expect("TARGET must be set");
    let wrapper = Path::new("bindgen/wrapper.h");
    let (lvgl_functions, lvgl_vars, lvgl_types) = lvgl_allowlists::load();

    println!("cargo:rerun-if-changed={}", wrapper.display());
    println!("cargo:rerun-if-changed=src/lvgl/bindings.conf");
    println!("cargo:rerun-if-env-changed=ZEPHYR_BASE");
    println!("cargo:rerun-if-env-changed=INCLUDE_DIRS");
    println!("cargo:rerun-if-env-changed=INCLUDE_DEFINES");
    println!("cargo:rerun-if-env-changed=BINARY_DIR_INCLUDE_GENERATED");

    // Re-run bindgen whenever autoconf.h changes (Kconfig options affecting widget enables).
    if let Ok(gen_dir) = env::var("BINARY_DIR_INCLUDE_GENERATED") {
        println!("cargo:rerun-if-changed={}/zephyr/autoconf.h", gen_dir);
    }

    let mut builder = bindgen::Builder::default()
        .header(wrapper.to_string_lossy())
        .use_core()
        .ctypes_prefix("core::ffi")
        .clang_arg(format!("--target={}", target))
        .allowlist_function(&lvgl_functions)
        .allowlist_var(&lvgl_vars)
        .generate_comments(false);

    if !lvgl_types.is_empty() {
        builder = builder.allowlist_type(&lvgl_types);
    }

    builder = define_args(builder, "-I", "INCLUDE_DIRS");
    builder = define_args(builder, "-D", "INCLUDE_DEFINES");

    // Bindgen-specific header stubs (e.g. sys/errno.h) live under bindgen/sys.
    let bindgen_include = Path::new(env!("CARGO_MANIFEST_DIR")).join("bindgen");
    builder = builder
        .clang_arg(format!("-I{}/lib/libc/minimal/include", zephyr_base))
        .clang_arg(format!("-I{}/modules/lvgl/include", zephyr_base))
        .clang_arg(format!("-I{}", bindgen_include.display()));

    let bindings = builder.generate().expect("Unable to generate bindings");
    bindings
        .write_to_file(Path::new(&out_dir).join("bindings.rs"))
        .expect("Failed to write bindings");
}

fn define_args(bindings: Builder, prefix: &str, var_name: &str) -> Builder {
    let text = match env::var(var_name) {
        Ok(val) => val,
        Err(_) => return bindings,
    };
    let mut bindings = bindings;
    for entry in text.split(&[' ', ';']) {
        if entry.is_empty() {
            continue;
        }
        let arg = format!("{}{}", prefix, entry);
        bindings = bindings.clang_arg(arg);
    }
    bindings
}
