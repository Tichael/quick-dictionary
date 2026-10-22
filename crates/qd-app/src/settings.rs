//! User settings, stored as JSON in the app's config folder.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Global shortcut that shows the lookup window, e.g. "Ctrl+Alt+D".
    pub shortcut: String,
    /// Start the app (in the tray) when the user logs in.
    pub start_with_system: bool,
    /// Hide the lookup window when it loses focus.
    pub hide_on_blur: bool,
    /// Clear the previous query when the window opens, instead of keeping
    /// it selected so typing replaces it.
    pub clear_on_show: bool,
    pub position: WindowPosition,
    pub theme: Theme,
    /// How many results to show for a query.
    pub max_results: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WindowPosition {
    /// Center the lookup window on the screen every time it opens.
    Center,
    /// Open the lookup window where it was last moved.
    Remember,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            shortcut: "Ctrl+Alt+D".into(),
            start_with_system: true,
            hide_on_blur: true,
            clear_on_show: false,
            position: WindowPosition::Center,
            theme: Theme::System,
            max_results: 12,
        }
    }
}

pub fn settings_file(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

impl Settings {
    /// Reads the settings file. A missing or unreadable file gives the
    /// defaults, and unknown or missing fields fall back to their defaults.
    pub fn load(file: &Path) -> Settings {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, file: &Path) -> std::io::Result<()> {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(tmp, file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_files_keep_defaults() {
        let settings: Settings = serde_json::from_str(r#"{"shortcut": "Ctrl+Shift+K"}"#).unwrap();
        assert_eq!(settings.shortcut, "Ctrl+Shift+K");
        assert_eq!(settings.max_results, Settings::default().max_results);
    }

    #[test]
    fn round_trip() {
        let dir = std::env::temp_dir().join(format!("qd-settings-test-{}", std::process::id()));
        let file = settings_file(&dir);
        let settings = Settings {
            hide_on_blur: false,
            theme: Theme::Dark,
            ..Settings::default()
        };
        settings.save(&file).unwrap();
        assert_eq!(Settings::load(&file), settings);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unreadable_file_gives_defaults() {
        assert_eq!(
            Settings::load(Path::new("/nonexistent/settings.json")),
            Settings::default()
        );
    }
}
