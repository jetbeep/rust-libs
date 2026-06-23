//! Catalog of parcel-locker simulator layouts.
//!
//! A catalog can be built from either a single layout file (legacy) or a
//! directory containing one `<name>.json` per layout plus an optional
//! `default.txt` naming the default.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::config::{validate_board_lock, CellConfig, LockerConfig};

#[derive(Debug, Clone)]
pub struct Layout {
    pub name: String,
    pub lockers: Vec<LockerConfig>,
}

#[derive(Debug, Clone)]
pub struct LayoutCatalog {
    pub layouts: Vec<Layout>,
    pub default_name: String,
}

impl LayoutCatalog {
    pub fn get(&self, name: &str) -> Option<&Layout> {
        self.layouts.iter().find(|l| l.name == name)
    }
}

#[derive(Debug)]
pub enum LoadError {
    Io(String),
    Parse(String),
    NoLayouts(String),
    OverrideNotFound { name: String, available: Vec<String> },
}

impl std::error::Error for LoadError {}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(m) => write!(f, "io: {}", m),
            LoadError::Parse(m) => write!(f, "parse: {}", m),
            LoadError::NoLayouts(m) => write!(f, "no layouts: {}", m),
            LoadError::OverrideNotFound { name, available } => write!(
                f,
                "--simulator-layout '{}' not found; available: [{}]",
                name,
                available.join(", ")
            ),
        }
    }
}

pub fn load_catalog(path: &Path, cli_override: Option<&str>) -> Result<LayoutCatalog, LoadError> {
    if path.is_dir() {
        load_directory(path, cli_override)
    } else {
        if cli_override.is_some() {
            log::warn!(
                "simulator: --simulator-layout ignored because --simulator-config '{}' is a file, not a directory",
                path.display()
            );
        }
        load_single_file(path)
    }
}

fn load_single_file(path: &Path) -> Result<LayoutCatalog, LoadError> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| LoadError::Io(format!("reading {}: {}", path.display(), e)))?;
    let lockers = parse_single_layout_file(&json)
        .map_err(|e| LoadError::Parse(format!("{}: {}", path.display(), e)))?;
    validate_layout(&lockers)
        .map_err(|e| LoadError::Parse(format!("{}: {}", path.display(), e)))?;

    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("default")
        .to_string();

    Ok(LayoutCatalog {
        default_name: name.clone(),
        layouts: vec![Layout { name, lockers }],
    })
}

#[derive(Debug, Deserialize)]
struct SingleLayoutFile {
    lockers: Vec<LockerConfig>,
    #[serde(default)]
    device_settings: Option<serde_json::Value>,
}

fn parse_single_layout_file(json: &str) -> Result<Vec<LockerConfig>, serde_json::Error> {
    let parsed: SingleLayoutFile = serde_json::from_str(json)?;
    if parsed.device_settings.is_none() {
        log::debug!("simulator: wrapped config has no device_settings section");
    }
    Ok(parsed.lockers)
}

fn load_directory(dir: &Path, cli_override: Option<&str>) -> Result<LayoutCatalog, LoadError> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| LoadError::Io(format!("reading dir {}: {}", dir.display(), e)))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("json"))
                .unwrap_or(false)
        })
        .collect();
    entries.sort();

    let mut layouts: Vec<Layout> = Vec::new();
    for path in entries {
        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        match load_one(&path) {
            Ok(lockers) => layouts.push(Layout { name, lockers }),
            Err(reason) => {
                log::warn!("simulator: layout \"{}\" rejected — {}", name, reason);
            }
        }
    }

    if layouts.is_empty() {
        return Err(LoadError::NoLayouts(format!(
            "no valid *.json layouts in {}",
            dir.display()
        )));
    }

    let default_name = pick_default(dir, &layouts, cli_override)?;
    Ok(LayoutCatalog { layouts, default_name })
}

fn load_one(path: &Path) -> Result<Vec<LockerConfig>, String> {
    let json = std::fs::read_to_string(path).map_err(|e| format!("read: {}", e))?;
    let lockers: Vec<LockerConfig> =
        serde_json::from_str(&json).map_err(|e| format!("parse: {}", e))?;
    validate_layout(&lockers)?;
    Ok(lockers)
}

fn validate_layout(lockers: &[LockerConfig]) -> Result<(), String> {
    if lockers.is_empty() {
        return Err("layout must contain at least one locker".to_string());
    }
    for (li, locker) in lockers.iter().enumerate() {
        match (&locker.cells, &locker.columns) {
            (Some(_), Some(_)) | (None, None) => {
                return Err(format!(
                    "locker[{}]: exactly one of `cells` or `columns` must be set",
                    li
                ));
            }
            (Some(cells), None) => {
                if cells.is_empty() {
                    return Err(format!("locker[{}]: `cells` must not be empty", li));
                }
                for cell in cells {
                    validate_cell(cell)?;
                }
            }
            (None, Some(columns)) => {
                if columns.is_empty() {
                    return Err(format!("locker[{}]: `columns` must not be empty", li));
                }
                for (ci, col) in columns.iter().enumerate() {
                    if col.cells.is_empty() {
                        return Err(format!(
                            "locker[{}].columns[{}]: `cells` must not be empty",
                            li, ci
                        ));
                    }
                    for cell in &col.cells {
                        validate_cell(cell)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_cell(cell: &CellConfig) -> Result<(), String> {
    if let Some(columns) = &cell.columns {
        if columns.is_empty() {
            return Err("cell has empty columns array".to_string());
        }
        for col in columns {
            validate_board_lock(col.board_id, col.lock_id)?;
        }
        Ok(())
    } else if let (Some(b), Some(l)) = (cell.board_id, cell.lock_id) {
        validate_board_lock(b, l)
    } else if cell.board_id.is_none() && cell.lock_id.is_none() {
        // Filler / service slot: no board/lock and no columns. Renders as
        // an inert spacer to keep column-major layouts aligned.
        Ok(())
    } else {
        Err("cell has partial (board_id, lock_id) — provide both or neither".to_string())
    }
}

fn pick_default(
    dir: &Path,
    layouts: &[Layout],
    cli_override: Option<&str>,
) -> Result<String, LoadError> {
    let names: Vec<String> = layouts.iter().map(|l| l.name.clone()).collect();

    if let Some(name) = cli_override {
        if layouts.iter().any(|l| l.name == name) {
            return Ok(name.to_string());
        }
        return Err(LoadError::OverrideNotFound {
            name: name.to_string(),
            available: names,
        });
    }

    let default_file = dir.join("default.txt");
    if default_file.exists() {
        match std::fs::read_to_string(&default_file) {
            Ok(contents) => {
                let wanted = contents.trim().to_string();
                if layouts.iter().any(|l| l.name == wanted) {
                    return Ok(wanted);
                }
                log::warn!(
                    "simulator: default.txt names \"{}\" but no such layout; falling back to first",
                    wanted
                );
            }
            Err(e) => {
                log::warn!("simulator: failed to read default.txt: {}", e);
            }
        }
    }

    Ok(layouts[0].name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_layout(dir: &Path, name: &str, json: &str) {
        let mut f = std::fs::File::create(dir.join(format!("{}.json", name))).unwrap();
        f.write_all(json.as_bytes()).unwrap();
    }

    const VALID_LAYOUT: &str = r#"
        [{"width":40,"depth":50,"open_check_policy":"always","cells":[
            {"cell_name":"1","height":30,"size":"M","board_id":1,"lock_id":1}
        ]}]
    "#;

    const WRAPPED_VALID_LAYOUT: &str = r#"
        {"lockers":[{"width":40,"depth":50,"open_check_policy":"always","cells":[
            {"cell_name":"1","height":30,"size":"M","board_id":1,"lock_id":1}
        ]}],"device_settings":{"display":{"theme":"light"}}}
    "#;

    const INVALID_LAYOUT: &str = r#"
        [{"width":40,"depth":50,"open_check_policy":"always","cells":[
            {"cell_name":"X","height":30,"size":"M","board_id":0,"lock_id":5}
        ]}]
    "#;

    const EMPTY_COLUMNS_LAYOUT: &str = r#"
        {"lockers":[{"width":40,"depth":50,"open_check_policy":"always","cells":[
            {"cell_name":"row","height":30,"size":"M","columns":[]}
        ]}]}
    "#;

    #[test]
    fn single_file_mode_errors_on_empty_columns_array() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("empty_cols.json");
        std::fs::write(&file, EMPTY_COLUMNS_LAYOUT).unwrap();
        let err = load_catalog(&file, None).unwrap_err();
        assert!(matches!(err, LoadError::Parse(_)), "got {:?}", err);
    }

    #[test]
    fn directory_mode_loads_uppercase_extension() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("alpha.JSON"), VALID_LAYOUT).unwrap();
        let cat = load_catalog(tmp.path(), None).unwrap();
        assert_eq!(cat.layouts.len(), 1);
        assert_eq!(cat.layouts[0].name, "alpha");
    }

    #[test]
    fn single_file_mode_loads_one_named_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("solo.json");
        std::fs::write(&file, WRAPPED_VALID_LAYOUT).unwrap();
        let cat = load_catalog(&file, None).unwrap();
        assert_eq!(cat.layouts.len(), 1);
        assert_eq!(cat.default_name, "solo");
        assert_eq!(cat.layouts[0].name, "solo");
    }

    #[test]
    fn single_file_mode_loads_wrapped_object_with_device_settings() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("wrapped.json");
        let wrapped = WRAPPED_VALID_LAYOUT;
        std::fs::write(&file, wrapped).unwrap();

        let cat = load_catalog(&file, None).unwrap();
        assert_eq!(cat.layouts.len(), 1);
        assert_eq!(cat.default_name, "wrapped");
        assert_eq!(cat.layouts[0].name, "wrapped");
    }

    #[test]
    fn single_file_mode_rejects_legacy_array_format() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("legacy.json");
        std::fs::write(&file, VALID_LAYOUT).unwrap();

        let err = load_catalog(&file, None).unwrap_err();
        assert!(matches!(err, LoadError::Parse(_)), "got {:?}", err);
    }

    #[test]
    fn single_file_mode_errors_on_invalid_cell() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("bad.json");
        let wrapped_bad = format!("{{\"lockers\":{}}}", INVALID_LAYOUT);
        std::fs::write(&file, wrapped_bad).unwrap();
        let err = load_catalog(&file, None).unwrap_err();
        assert!(matches!(err, LoadError::Parse(_)), "got {:?}", err);
    }

    #[test]
    fn directory_mode_loads_all_json_files_sorted() {
        let tmp = tempfile::tempdir().unwrap();
        write_layout(tmp.path(), "b-second", VALID_LAYOUT);
        write_layout(tmp.path(), "a-first", VALID_LAYOUT);
        let cat = load_catalog(tmp.path(), None).unwrap();
        let names: Vec<_> = cat.layouts.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, vec!["a-first", "b-second"]);
        assert_eq!(cat.default_name, "a-first");
    }

    #[test]
    fn directory_mode_skips_invalid_layout_with_warning() {
        let tmp = tempfile::tempdir().unwrap();
        write_layout(tmp.path(), "good", VALID_LAYOUT);
        write_layout(tmp.path(), "bad", INVALID_LAYOUT);
        let cat = load_catalog(tmp.path(), None).unwrap();
        let names: Vec<_> = cat.layouts.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, vec!["good"]);
    }

    #[test]
    fn empty_directory_is_error() {
        let tmp = tempfile::tempdir().unwrap();
        let err = load_catalog(tmp.path(), None).unwrap_err();
        assert!(matches!(err, LoadError::NoLayouts(_)));
    }

    #[test]
    fn default_txt_picks_named_layout() {
        let tmp = tempfile::tempdir().unwrap();
        write_layout(tmp.path(), "alpha", VALID_LAYOUT);
        write_layout(tmp.path(), "beta", VALID_LAYOUT);
        std::fs::write(tmp.path().join("default.txt"), "beta\n").unwrap();
        let cat = load_catalog(tmp.path(), None).unwrap();
        assert_eq!(cat.default_name, "beta");
    }

    #[test]
    fn default_txt_unknown_falls_back_to_first() {
        let tmp = tempfile::tempdir().unwrap();
        write_layout(tmp.path(), "alpha", VALID_LAYOUT);
        write_layout(tmp.path(), "beta", VALID_LAYOUT);
        std::fs::write(tmp.path().join("default.txt"), "gamma\n").unwrap();
        let cat = load_catalog(tmp.path(), None).unwrap();
        assert_eq!(cat.default_name, "alpha");
    }

    #[test]
    fn cli_override_picks_named_layout() {
        let tmp = tempfile::tempdir().unwrap();
        write_layout(tmp.path(), "alpha", VALID_LAYOUT);
        write_layout(tmp.path(), "beta", VALID_LAYOUT);
        let cat = load_catalog(tmp.path(), Some("beta")).unwrap();
        assert_eq!(cat.default_name, "beta");
    }

    #[test]
    fn cli_override_unknown_is_error() {
        let tmp = tempfile::tempdir().unwrap();
        write_layout(tmp.path(), "alpha", VALID_LAYOUT);
        let err = load_catalog(tmp.path(), Some("missing")).unwrap_err();
        assert!(matches!(err, LoadError::OverrideNotFound { .. }));
    }

    #[test]
    fn column_major_layout_parses_and_validates() {
        let json = r#"[{
            "width": 40, "depth": 50,
            "columns": [
                { "width": 40, "cells": [
                    { "cell_name": "1", "height": 50, "size": "M", "board_id": 1, "lock_id": 1 },
                    { "cell_name": "2", "height": 50, "size": "M", "board_id": 1, "lock_id": 2 }
                ]}
            ]
        }]"#;
        let lockers: Vec<LockerConfig> = serde_json::from_str(json).expect("parse");
        validate_layout(&lockers).expect("validate");
        let cols = lockers[0].columns.as_ref().unwrap();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].cells.len(), 2);
        assert_eq!(cols[0].width, 40);
    }

    #[test]
    fn rejects_locker_with_both_cells_and_columns() {
        let json = r#"[{
            "width": 40, "depth": 50,
            "cells": [{ "cell_name": "1", "height": 30, "size": "S", "board_id": 1, "lock_id": 1 }],
            "columns": [{ "width": 40, "cells": [
                { "cell_name": "2", "height": 30, "size": "S", "board_id": 1, "lock_id": 2 }
            ]}]
        }]"#;
        let lockers: Vec<LockerConfig> = serde_json::from_str(json).expect("parse");
        let err = validate_layout(&lockers).unwrap_err();
        assert!(err.contains("exactly one of"), "got: {}", err);
    }

    #[test]
    fn rejects_locker_with_neither_cells_nor_columns() {
        let json = r#"[{ "width": 40, "depth": 50 }]"#;
        let lockers: Vec<LockerConfig> = serde_json::from_str(json).expect("parse");
        let err = validate_layout(&lockers).unwrap_err();
        assert!(err.contains("exactly one of"), "got: {}", err);
    }
}
