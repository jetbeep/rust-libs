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
    println!("cargo:rustc-check-cfg=cfg(gen_style_props)");

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
            let include_dirs = env::var("LVGL_INCLUDE_DIRS").expect("checked above");
            // Derive the LVGL style-property ids from the *real* headers this
            // simulator links against, instead of trusting hand-maintained
            // constants that silently rot when the vendored LVGL is repinned.
            generate_style_props(&include_dirs, &out_dir);
            println!("cargo:rustc-cfg=gen_style_props");
            run_abi_probe();
        }
        return;
    }

    generate_bindings();
}

/// Desktop-sim ABI guard: compile `build/abi_probe.c` against the real LVGL
/// headers (paths from LVGL_INCLUDE_DIRS). The probe is all `_Static_assert`s
/// — constant/layout drift between the Rust-side mirrors and the vendored
/// LVGL fails the build with a precise message instead of misbehaving at
/// runtime.
fn run_abi_probe() {
    let include_dirs = env::var("LVGL_INCLUDE_DIRS").expect("checked by caller");
    println!("cargo:rerun-if-changed=build/abi_probe.c");
    // The probe's asserted values mirror src/c_bindings.rs / static_style.rs;
    // rerun when either side of the contract changes.
    println!("cargo:rerun-if-changed=src/c_bindings.rs");
    println!("cargo:rerun-if-changed=src/lvgl/static_style.rs");
    println!("cargo:rerun-if-env-changed=LVGL_INCLUDE_DIRS");
    println!("cargo:rerun-if-env-changed=LV_CONF_DIR");

    let mut build = cc::Build::new();
    build.file("build/abi_probe.c");
    for dir in include_dirs
        .split(&[';', ' ', '\t', '\n'])
        .filter(|d| !d.is_empty())
    {
        build.include(dir);
    }
    if let Ok(conf_dir) = env::var("LV_CONF_DIR") {
        build.include(conf_dir);
    }
    build
        .warnings(false)
        .compile("jb_dsl_abi_probe");
}

/// Parse the real `lv_style.h` (located under `include_dirs`) and emit
/// `$OUT_DIR/lv_style_props.rs` with the exact `lv_style_prop_t` ids for the
/// LVGL version this crate compiles against. Desktop wrapper: drift here is a
/// hard error, so panic on any failure.
fn generate_style_props(include_dirs: &str, out_dir: &str) {
    emit_style_props(include_dirs, out_dir)
        .unwrap_or_else(|e| panic!("style-prop generation failed: {e}"));
}

/// Core style-prop generation shared by the desktop and firmware builds.
///
/// Static (const) styles encode each property by its numeric id in a
/// `lv_style_const_prop_t` array; those ids are *renumbered* between LVGL
/// releases (e.g. `LV_STYLE_TEXT_FONT` is 77 in 9.6 but 90 in 9.3). Hardcoding
/// them in Rust silently breaks every static style the moment the vendored
/// LVGL is repinned. Deriving them from the header keeps both the simulator
/// and the firmware correct for whatever LVGL they actually compile against.
///
/// Returns `Err` (instead of panicking) so the firmware build can fall back to
/// the hand-maintained ids if the header can't be located.
fn emit_style_props(include_dirs: &str, out_dir: &str) -> Result<(), String> {
    println!("cargo:rerun-if-changed=src/lvgl/static_style.rs");

    // 1. Master list: every `pub const LV_STYLE_<NAME>: StyleProp = <N>;` in our
    //    source, paired with its hand-maintained fallback value.
    let src = fs::read_to_string("src/lvgl/static_style.rs")
        .map_err(|e| format!("read static_style.rs: {e}"))?;
    let master = parse_master_style_props(&src);
    if !master.iter().any(|(n, _)| n == "LV_STYLE_TEXT_FONT") {
        return Err("failed to parse the style-prop master list from static_style.rs".into());
    }

    // 2. Locate and parse the real lv_style.h enum.
    let header = find_lv_style_header(include_dirs)
        .ok_or_else(|| "could not locate lv_style.h under include dirs".to_string())?;
    println!("cargo:rerun-if-changed={}", header.display());
    let header_src =
        fs::read_to_string(&header).map_err(|e| format!("read {}: {e}", header.display()))?;
    let parsed = parse_style_prop_enum(&header_src);
    if !parsed.contains_key("LV_STYLE_TEXT_FONT") {
        return Err(format!(
            "failed to parse lv_style_prop_t from {}",
            header.display()
        ));
    }

    // 3. Emit, preferring the real header value, falling back to our default for
    //    properties that don't exist in this LVGL version.
    let mut out = String::new();
    out.push_str("// @generated by build.rs::emit_style_props from the real lv_style.h\n");
    out.push_str("// this crate compiles against. Do not edit by hand.\n");
    for (name, fallback) in &master {
        let val = parsed.get(name).copied().unwrap_or(*fallback);
        out.push_str(&format!("pub const {}: StyleProp = {};\n", name, val));
    }
    fs::write(Path::new(out_dir).join("lv_style_props.rs"), out)
        .map_err(|e| format!("write lv_style_props.rs: {e}"))?;
    Ok(())
}

fn parse_master_style_props(src: &str) -> Vec<(String, u8)> {
    let mut master = Vec::new();
    for line in src.lines() {
        let l = line.trim();
        let rest = match l.strip_prefix("pub const LV_STYLE_") {
            Some(r) => r,
            None => continue,
        };
        if let Some((name_tail, val_tail)) = rest.split_once(": StyleProp = ") {
            let name = format!("LV_STYLE_{}", name_tail.trim());
            let val_str = val_tail.trim().trim_end_matches(';').trim();
            if let Ok(v) = val_str.parse::<u8>() {
                master.push((name, v));
            }
        }
    }
    master
}

fn find_lv_style_header(include_dirs: &str) -> Option<std::path::PathBuf> {
    const CANDIDATES: [&str; 4] = [
        "src/misc/lv_style.h",
        "misc/lv_style.h",
        "lv_style.h",
        "lvgl/src/misc/lv_style.h",
    ];
    for dir in include_dirs.split([' ', ';']).filter(|d| !d.is_empty()) {
        for sub in CANDIDATES {
            let p = Path::new(dir).join(sub);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Resolve the `lv_style_prop_t` enum members to their numeric values, honoring
/// explicit `= N` assignments (decimal or hex) and incrementing otherwise.
fn parse_style_prop_enum(src: &str) -> std::collections::BTreeMap<String, u8> {
    let mut map = std::collections::BTreeMap::new();
    let mut in_enum = false;
    let mut next: i64 = 0;
    for line in src.lines() {
        let t = line.trim();
        // The enum body starts at the LV_STYLE_PROP_INV member. Guard against
        // the LV_STYLE_CONST_PROPS_END macro (which also mentions PROP_INV) by
        // requiring the line to *start* with the member name.
        if !in_enum {
            if t.starts_with("LV_STYLE_PROP_INV") {
                in_enum = true;
            } else {
                continue;
            }
        }
        if let Some((name, val)) = parse_enum_member(t) {
            let cur = val.unwrap_or(next);
            if (0..=255).contains(&cur) {
                map.insert(name, cur as u8);
            }
            next = cur + 1;
        }
        if t.contains('}') {
            break;
        }
    }
    map
}

fn parse_enum_member(t: &str) -> Option<(String, Option<i64>)> {
    if !t.starts_with("LV_STYLE_") {
        return None;
    }
    let name_end = t
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(t.len());
    let name = t[..name_end].to_string();
    let rest = t[name_end..].trim_start();
    if let Some(after_eq) = rest.strip_prefix('=') {
        let tok: String = after_eq
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == 'x' || *c == 'X')
            .collect();
        Some((name, parse_int(&tok)))
    } else {
        // Implicit value (`NAME,` / `NAME`); the caller supplies the running counter.
        Some((name, None))
    }
}

fn parse_int(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(h, 16).ok()
    } else {
        s.parse::<i64>().ok()
    }
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
        .prepend_enum_name(false)
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

    // Derive LVGL style-property ids from the real headers (mirrors the desktop
    // path) so static const styles use the correct numeric ids for THIS LVGL,
    // not the hand-maintained fallback table (which tracks a different LVGL
    // version and otherwise leaves every static style writing to the wrong
    // property on device). If the header can't be located, keep the fallback.
    let mut style_dirs = env::var("INCLUDE_DIRS").unwrap_or_default();
    for cand in [
        format!("{zephyr_base}/../modules/lib/gui/lvgl"),
        format!("{zephyr_base}/modules/lvgl"),
    ] {
        style_dirs.push(';');
        style_dirs.push_str(&cand);
    }
    match emit_style_props(&style_dirs, &out_dir) {
        Ok(()) => println!("cargo:rustc-cfg=gen_style_props"),
        Err(e) => {
            println!("cargo:warning=lvgl-dsl: using hardcoded style-prop ids ({e})")
        }
    }
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
