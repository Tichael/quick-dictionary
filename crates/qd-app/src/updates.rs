//! Updating the app itself. The release workflow publishes a signed
//! installer and a `latest.json` describing it with each GitHub release;
//! the updater plugin reads that file, checks the signature against the
//! public key in tauri.conf.json, and runs the installer.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Url};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::i18n::{self, Text, text};
use crate::{show_settings_section, ui_lang};

/// Where the app looks for a newer version. QD_UPDATES_URL overrides it
/// (testing): the address of a `latest.json`.
const UPDATES_ENV: &str = "QD_UPDATES_URL";

/// How long after starting the first check waits (the network may not be
/// up yet right after logging in), then how often the app checks again.
const FIRST_CHECK: Duration = Duration::from_secs(10);
const CHECK_EVERY: Duration = Duration::from_secs(24 * 60 * 60);

/// What the settings window shows about updates.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Status {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available {
        version: String,
    },
    Downloading {
        received: u64,
        total: u64,
    },
    Installing,
    Failed {
        message: String,
    },
}

#[derive(Default)]
pub struct Updates {
    status: Mutex<Status>,
    /// The update found by the last check, until it is installed.
    pending: Mutex<Option<Update>>,
    /// The version the user answered "Later" to: not asked again.
    declined: Mutex<Option<String>>,
}

impl Updates {
    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }
}

fn set_status(app: &AppHandle, status: Status) {
    *app.state::<Updates>().status.lock().unwrap() = status.clone();
    let _ = app.emit("update-status", status);
}

fn busy(app: &AppHandle) -> bool {
    matches!(
        app.state::<Updates>().status(),
        Status::Checking | Status::Downloading { .. } | Status::Installing
    )
}

/// Looks for a newer version and remembers it. Errors are in the
/// interface language.
pub async fn check(app: &AppHandle) -> Result<Status, String> {
    if busy(app) {
        return Ok(app.state::<Updates>().status());
    }
    let ui = ui_lang(app);
    set_status(app, Status::Checking);
    let found = async {
        let mut builder = app.updater_builder();
        if let Ok(url) = std::env::var(UPDATES_ENV) {
            let url = Url::parse(&url).map_err(|e| e.to_string())?;
            builder = builder.endpoints(vec![url]).map_err(|e| e.to_string())?;
        }
        let updater = builder.build().map_err(|e| e.to_string())?;
        updater.check().await.map_err(|e| e.to_string())
    }
    .await;
    let status = match found {
        Ok(found) => {
            let version = found.as_ref().map(|update| update.version.clone());
            *app.state::<Updates>().pending.lock().unwrap() = found;
            match version {
                Some(version) => Status::Available { version },
                None => Status::UpToDate,
            }
        }
        Err(e) => Status::Failed {
            message: i18n::update_check_error(ui, e),
        },
    };
    set_status(app, status.clone());
    match status {
        Status::Failed { message } => Err(message),
        status => Ok(status),
    }
}

/// Downloads and runs the installer of the update found by `check`. On
/// Windows the installer replaces the app, which exits, and starts it
/// again.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    if busy(app) {
        return Ok(());
    }
    let ui = ui_lang(app);
    let Some(update) = app.state::<Updates>().pending.lock().unwrap().clone() else {
        return Err(i18n::update_install_error(ui, "no update to install"));
    };
    set_status(
        app,
        Status::Downloading {
            received: 0,
            total: 0,
        },
    );
    let (events, mut received, mut last_percent) = (app.clone(), 0u64, None);
    let installed = update
        .download_and_install(
            move |chunk, total| {
                received += chunk as u64;
                let total = total.unwrap_or(0);
                // One event per percent is plenty for a progress bar.
                let percent = received * 100 / total.max(1);
                if last_percent != Some(percent) {
                    last_percent = Some(percent);
                    set_status(&events, Status::Downloading { received, total });
                }
            },
            || set_status(app, Status::Installing),
        )
        .await;
    match installed {
        // Only reached outside Windows: the new version runs after a restart.
        Ok(()) => app.restart(),
        Err(e) => {
            let message = i18n::update_install_error(ui, e);
            set_status(
                app,
                Status::Failed {
                    message: message.clone(),
                },
            );
            Err(message)
        }
    }
}

/// Checks for updates in the background, now and then once a day. When a
/// new version is found, asks whether to install it (once per version).
pub fn check_in_background(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            match tauri::async_runtime::block_on(check(&app)) {
                Ok(Status::Available { version }) => offer(&app, version),
                Ok(_) => {}
                Err(e) => eprintln!("{e}"),
            }
            std::thread::sleep(CHECK_EVERY);
        }
    });
}

/// Asks whether to install `version` now. If so, shows the About section
/// of the settings window, where the download's progress shows, and
/// installs it.
fn offer(app: &AppHandle, version: String) {
    let updates = app.state::<Updates>();
    if updates.declined.lock().unwrap().as_deref() == Some(version.as_str()) {
        return;
    }
    let lang = ui_lang(app);
    let current = app.package_info().version.to_string();
    let confirmed = app
        .dialog()
        .message(i18n::update_question(lang, &version, &current))
        .title(text(lang, Text::UpdateTitle))
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            text(lang, Text::UpdateInstall).into(),
            text(lang, Text::UpdateLater).into(),
        ))
        .blocking_show();
    if !confirmed {
        *updates.declined.lock().unwrap() = Some(version);
        return;
    }
    show_settings_section(app, "about");
    if let Err(e) = tauri::async_runtime::block_on(install(app)) {
        eprintln!("{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The settings window reads these names (settings.js, renderUpdate).
    #[test]
    fn status_serializes_as_the_settings_window_expects() {
        let json = |status| serde_json::to_value(status).unwrap();
        assert_eq!(
            json(Status::UpToDate),
            serde_json::json!({"state": "upToDate"})
        );
        assert_eq!(
            json(Status::Downloading {
                received: 5,
                total: 10
            }),
            serde_json::json!({"state": "downloading", "received": 5, "total": 10})
        );
        assert_eq!(
            json(Status::Available {
                version: "1.2.3".into()
            }),
            serde_json::json!({"state": "available", "version": "1.2.3"})
        );
    }

    /// The plugin reads its section of tauri.conf.json when the app starts:
    /// a mistake there would stop the app from starting at all.
    #[test]
    fn updater_config_is_valid() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let config: tauri_plugin_updater::Config =
            serde_json::from_value(conf["plugins"]["updater"].clone()).unwrap();
        assert!(config.require_signed_version);
        assert_eq!(config.endpoints.len(), 1);
        assert!(config.endpoints[0].as_str().ends_with("/latest.json"));
    }
}
