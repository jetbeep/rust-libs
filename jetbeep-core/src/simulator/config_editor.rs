//! Live editing of the simulated device's `user_params` from the Locker
//! Simulator window.
//!
//! The simulator's device settings come from the wrapped JSON config file at
//! `bus::simulator_config_path()`, specifically
//! `device_settings.user_settings.user_params_json`. This module reads that
//! object (for seeding the editor) and writes an edited object back to the
//! file, then asks the app to hot-reload its profile so the change takes
//! effect without a restart.
//!
//! All functions are simulator-only and operate on `std`.

use std::cell::RefCell;
use std::path::Path;

use serde_json::Value;

/// Registered by the app so a successful save can hot-reload the active
/// profile. Thread-local because the simulator runs single-threaded on the
/// LVGL/workq thread.
thread_local! {
    static RELOAD_HOOK: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

/// Register the profile hot-reload hook. The app passes a closure that
/// re-reads device settings and re-applies the active profile (e.g.
/// `App::start_async_profile_load`).
pub fn set_profile_reload_hook(hook: Box<dyn Fn()>) {
    RELOAD_HOOK.with(|h| *h.borrow_mut() = Some(hook));
}

/// Invoke the registered reload hook, if any. No-op when unset.
pub fn trigger_profile_reload() {
    RELOAD_HOOK.with(|h| {
        if let Some(hook) = h.borrow().as_ref() {
            hook();
        }
    });
}

/// Pretty-printed current `user_params` object from the active simulator config
/// file, suitable for seeding the editor. Returns `None` when no config path is
/// set, the path is a directory (legacy `simulator_layouts/` mode), the file is
/// unreadable/malformed, or the `user_params_json` key is absent.
pub fn current_user_params_json() -> Option<String> {
    let path = crate::bus::simulator_config_path()?;
    read_user_params_from_path(Path::new(&path))
}

/// Parse `text` as a JSON object and write it back to the active simulator
/// config file at `device_settings.user_settings.user_params_json`, preserving
/// the rest of the file. Returns a human-readable error string on failure.
///
/// On success the caller is expected to call [`trigger_profile_reload`].
pub fn save_user_params_json(text: &str) -> Result<(), String> {
    let path = crate::bus::simulator_config_path()
        .ok_or_else(|| "simulator config path is not set".to_string())?;
    save_user_params_to_path(Path::new(&path), text)
}

// ── Pure helpers (path-injected, unit-tested) ──────────────────────────────

fn read_user_params_from_path(path: &Path) -> Option<String> {
    if path.is_dir() {
        return None;
    }
    let raw = std::fs::read_to_string(path).ok()?;
    let root: Value = serde_json::from_str(&raw).ok()?;
    let params = root
        .get("device_settings")?
        .get("user_settings")?
        .get("user_params_json")?;
    serde_json::to_string_pretty(params).ok()
}

fn save_user_params_to_path(path: &Path, text: &str) -> Result<(), String> {
    let new_params: Value =
        serde_json::from_str(text).map_err(|e| format!("invalid JSON: {}", e))?;
    if !new_params.is_object() {
        return Err("user_params must be a JSON object".to_string());
    }

    if path.is_dir() {
        return Err(
            "editing requires a single-file simulator_config.json \
             (the config path is currently a layouts directory)"
                .to_string(),
        );
    }

    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("failed reading {}: {}", path.display(), e))?;
    let mut root: Value = serde_json::from_str(&raw)
        .map_err(|e| format!("failed parsing {}: {}", path.display(), e))?;

    set_user_params_json(&mut root, new_params)?;

    let mut out = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("failed serializing config: {}", e))?;
    out.push('\n');
    std::fs::write(path, out).map_err(|e| format!("failed writing {}: {}", path.display(), e))?;
    Ok(())
}

/// Set `device_settings.user_settings.user_params_json` to `params`, creating
/// the intermediate objects if absent. Fails if an existing intermediate value
/// is a non-object (so we never clobber unexpected structure).
fn set_user_params_json(root: &mut Value, params: Value) -> Result<(), String> {
    let root_obj = root
        .as_object_mut()
        .ok_or_else(|| "config root is not a JSON object".to_string())?;
    let device_settings = root_obj
        .entry("device_settings")
        .or_insert_with(|| Value::Object(Default::default()));
    let device_obj = device_settings
        .as_object_mut()
        .ok_or_else(|| "device_settings is not a JSON object".to_string())?;
    let user_settings = device_obj
        .entry("user_settings")
        .or_insert_with(|| Value::Object(Default::default()));
    let user_obj = user_settings
        .as_object_mut()
        .ok_or_else(|| "device_settings.user_settings is not a JSON object".to_string())?;
    user_obj.insert("user_params_json".to_string(), params);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("jb_config_editor_{}_{}.json", std::process::id(), name));
        p
    }

    fn write(path: &Path, contents: &str) {
        let mut f = std::fs::File::create(path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
    }

    const SAMPLE: &str = r#"{
      "device_settings": {
        "user_settings": {
          "user_params_json": { "locker_id": "old", "network_name": "OLX" }
        }
      },
      "lockers": [ { "width": 40 } ]
    }"#;

    #[test]
    fn round_trips_saved_object() {
        let path = temp_path("round_trip");
        write(&path, SAMPLE);

        save_user_params_to_path(&path, r#"{"locker_id":"new","network_type":"open"}"#).unwrap();

        let seeded = read_user_params_from_path(&path).unwrap();
        let params: Value = serde_json::from_str(&seeded).unwrap();
        assert_eq!(params["locker_id"], "new");
        assert_eq!(params["network_type"], "open");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn preserves_other_top_level_keys() {
        let path = temp_path("preserve");
        write(&path, SAMPLE);

        save_user_params_to_path(&path, r#"{"locker_id":"x"}"#).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let root: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(root["lockers"][0]["width"], 40);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rejects_invalid_json() {
        let path = temp_path("invalid");
        write(&path, SAMPLE);
        let err = save_user_params_to_path(&path, "{not json").unwrap_err();
        assert!(err.contains("invalid JSON"), "got: {}", err);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rejects_non_object_json() {
        let path = temp_path("nonobject");
        write(&path, SAMPLE);
        let err = save_user_params_to_path(&path, "[1,2,3]").unwrap_err();
        assert!(err.contains("must be a JSON object"), "got: {}", err);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn directory_path_is_guarded() {
        let dir = std::env::temp_dir();
        assert!(read_user_params_from_path(&dir).is_none());
        let err = save_user_params_to_path(&dir, "{}").unwrap_err();
        assert!(err.contains("single-file"), "got: {}", err);
    }

    #[test]
    fn creates_missing_intermediate_objects() {
        let path = temp_path("create_missing");
        write(&path, r#"{"lockers":[]}"#);

        save_user_params_to_path(&path, r#"{"locker_id":"z"}"#).unwrap();

        let seeded = read_user_params_from_path(&path).unwrap();
        let params: Value = serde_json::from_str(&seeded).unwrap();
        assert_eq!(params["locker_id"], "z");
        let _ = std::fs::remove_file(&path);
    }
}
