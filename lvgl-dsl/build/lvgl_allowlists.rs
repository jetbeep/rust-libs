use std::fs;
use std::path::Path;

pub(crate) fn load() -> (String, String, String) {
    let bindings_conf = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lvgl/bindings.conf");

    println!("cargo:rerun-if-changed={}", bindings_conf.display());

    let contents = fs::read_to_string(&bindings_conf)
        .unwrap_or_else(|err| panic!("Failed to read {}: {err}", bindings_conf.display()));

    let mut functions = Vec::new();
    let mut vars = Vec::new();
    let mut types = Vec::new();
    let mut current_section: Option<&str> = None;

    for line in contents.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        match trimmed {
            "[functions]" => current_section = Some("functions"),
            "[vars]" => current_section = Some("vars"),
            "[types]" => current_section = Some("types"),
            _ => match current_section {
                Some("functions") => functions.push(trimmed.to_owned()),
                Some("vars") => vars.push(trimmed.to_owned()),
                Some("types") => types.push(trimmed.to_owned()),
                _ => {
                    panic!(
                        "Malformed {}: entry outside of [functions]/[vars]/[types]: {trimmed}",
                        bindings_conf.display()
                    );
                }
            },
        }
    }

    assert!(
        !functions.is_empty(),
        "Malformed {}: [functions] section is missing or empty",
        bindings_conf.display()
    );
    assert!(
        !vars.is_empty(),
        "Malformed {}: [vars] section is missing or empty",
        bindings_conf.display()
    );

    (functions.join("|"), vars.join("|"), types.join("|"))
}
