use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set"));
    let proto_root = manifest_dir.join("../../proto-files");
    let device_settings_proto = proto_root.join("settings/common/device_settings.proto");
    let lock_statuses_proto = proto_root.join("bus/v2/poll_cmd/lock_statuses_get.proto");
    let modem_get_info_proto = proto_root.join("bus/v2/poll_cmd/modem_get_info.proto");
    let battery_get_info_proto = proto_root.join("bus/v2/poll_cmd/battery_get_info.proto");
    let version_info_proto = proto_root.join("bus/v2/poll_cmd/version_info.proto");
    let server_request_proto = proto_root.join("bus/v2/poll_cmd/server_request.proto");

    println!("cargo:rerun-if-changed={}", device_settings_proto.display());
    println!("cargo:rerun-if-changed={}", lock_statuses_proto.display());
    println!("cargo:rerun-if-changed={}", modem_get_info_proto.display());
    println!("cargo:rerun-if-changed={}", battery_get_info_proto.display());
    println!("cargo:rerun-if-changed={}", version_info_proto.display());
    println!("cargo:rerun-if-changed={}", server_request_proto.display());
    println!("cargo:rerun-if-changed={}", proto_root.join("settings/common").display());
    println!("cargo:rerun-if-changed={}", proto_root.join("bus/v2/poll_cmd").display());

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
