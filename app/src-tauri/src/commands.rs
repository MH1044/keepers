//! Tauri commands: thin, validated calls into the core.

use serde::Serialize;

/// What the window shows on start: `Keepers {version}`.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct AppInfo {
    pub version: String,
}

/// The value behind the `app_info` command, callable without a Tauri runtime.
pub fn app_info_value() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    app_info_value()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_returns_the_crate_version() {
        let info = app_info_value();
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.version, keepers_core::VERSION);
    }

    #[test]
    fn app_info_serialises_with_a_version_field() {
        let json = serde_json::to_value(app_info_value()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "version": env!("CARGO_PKG_VERSION") })
        );
    }
}
