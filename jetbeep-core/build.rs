use std::env;
use std::path::PathBuf;

use bindgen::Builder;

fn main() {
    if env::var("CARGO_FEATURE_PLATFORM_ZEPHYR").is_ok() {
        generate_zephyr_bindings();
    }

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"));
    let proto_root = manifest_dir.join("proto");
    if !proto_root
        .join("settings/common/device_settings.proto")
        .is_file()
        || !proto_root
            .join("bus/v2/poll_cmd/lock_statuses_get.proto")
            .is_file()
    {
        panic!(
            "public protocol definitions are missing from {}",
            proto_root.display()
        );
    }
    let device_settings_proto = proto_root.join("settings/common/device_settings.proto");
    let lock_statuses_proto = proto_root.join("bus/v2/poll_cmd/lock_statuses_get.proto");
    let modem_get_info_proto = proto_root.join("bus/v2/poll_cmd/modem_get_info.proto");
    let battery_get_info_proto = proto_root.join("bus/v2/poll_cmd/battery_get_info.proto");
    let version_info_proto = proto_root.join("bus/v2/poll_cmd/version_info.proto");
    let server_request_proto = proto_root.join("bus/v2/poll_cmd/server_request.proto");

    println!("cargo:rerun-if-changed={}", device_settings_proto.display());
    println!("cargo:rerun-if-changed={}", lock_statuses_proto.display());
    println!("cargo:rerun-if-changed={}", modem_get_info_proto.display());
    println!(
        "cargo:rerun-if-changed={}",
        battery_get_info_proto.display()
    );
    println!("cargo:rerun-if-changed={}", version_info_proto.display());
    println!("cargo:rerun-if-changed={}", server_request_proto.display());
    println!("cargo:rerun-if-changed={}", proto_root.display());

    let protoc = protoc_bin_vendored::protoc_bin_path().expect("failed to locate vendored protoc");
    env::set_var("PROTOC", protoc);

    let mut config = prost_build::Config::new();
    config.include_file("proto_mod.rs");
    config.message_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
    config.message_attribute(".", "#[serde(default, deny_unknown_fields)]");
    config.enum_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");

    config
        .compile_protos(
            &[
                device_settings_proto,
                lock_statuses_proto,
                modem_get_info_proto,
                battery_get_info_proto,
                version_info_proto,
                server_request_proto,
            ],
            &[proto_root],
        )
        .expect("failed to compile proto files for jetbeep-core");
}

fn generate_zephyr_bindings() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be set");
    let target = env::var("TARGET").expect("TARGET must be set");
    let zephyr_base = env::var("ZEPHYR_BASE").ok();
    let wrapper = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindgen/wrapper.h");

    println!("cargo:rerun-if-changed={}", wrapper.display());
    println!("cargo:rerun-if-env-changed=INCLUDE_DIRS");
    println!("cargo:rerun-if-env-changed=INCLUDE_DEFINES");
    println!("cargo:rerun-if-env-changed=BINARY_DIR_INCLUDE_GENERATED");

    let bindgen_include = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bindgen");

    let mut builder = Builder::default()
        .header(wrapper.to_string_lossy())
        .use_core()
        .ctypes_prefix("core::ffi")
        .clang_arg(format!("--target={}", target))
        .clang_arg(format!("-I{}", bindgen_include.display()))
        .generate_comments(false)
        .layout_tests(false);

    if let Some(zephyr_base) = zephyr_base {
        builder = builder
            .clang_arg(format!("-I{}/lib/libc/minimal/include", zephyr_base))
            .clang_arg(format!("-I{}/modules/lvgl/include", zephyr_base));
    }

    builder = define_args(builder, "-I", "INCLUDE_DIRS");
    builder = define_args(builder, "-D", "INCLUDE_DEFINES");

    if let Ok(generated_zephyr_include) = env::var("BINARY_DIR_INCLUDE_GENERATED") {
        builder = builder.clang_arg(format!("-include{}/autoconf.h", generated_zephyr_include));
    }

    let bindings = builder
        .generate()
        .expect("Unable to generate jetbeep-core bindings");
    bindings
        .write_to_file(PathBuf::from(out_dir).join("bindings.rs"))
        .expect("Failed to write jetbeep-core bindings");
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
        bindings = bindings.clang_arg(format!("{}{}", prefix, entry));
    }
    bindings
}
