//! Quick Dictionary: a tray app that opens a lookup window on a global
//! shortcut.

// No console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod downloads;
mod i18n;
mod library;
mod settings;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;

use qd_core::{SearchResult, Suggestion};
use serde::Serialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent, Wry,
};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_opener::OpenerExt;

use i18n::{Lang, Text, text};
use library::{Library, LibraryInfo};
use settings::{Settings, WindowPosition, settings_file};

const LOOKUP: &str = "lookup";
const SETTINGS: &str = "settings";

struct AppState {
    library: Library,
    settings: Mutex<Settings>,
    settings_file: PathBuf,
    /// The tray menu's items, relabeled when the interface language changes.
    tray_items: Mutex<Vec<(MenuItem<Wry>, Text)>>,
    /// Languages being downloaded, so a pack is not downloaded twice at once.
    downloading: Mutex<HashSet<String>>,
}

fn settings(app: &AppHandle) -> Settings {
    app.state::<AppState>().settings.lock().unwrap().clone()
}

/// The interface language now in use.
fn ui_lang(app: &AppHandle) -> Lang {
    Lang::resolve(settings(app).language)
}

/// Relabels what the app itself shows: tray menu and settings window title.
fn apply_language(app: &AppHandle) {
    let lang = ui_lang(app);
    for (item, label) in app.state::<AppState>().tray_items.lock().unwrap().iter() {
        let _ = item.set_text(text(lang, *label));
    }
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.set_title(text(lang, Text::SettingsTitle));
    }
}

// --- Windows -------------------------------------------------------------

fn show_lookup(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LOOKUP) else {
        return;
    };
    if settings(app).position == WindowPosition::Center {
        let _ = window.center();
    }
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    let _ = window.emit("lookup-shown", ());
}

fn toggle_lookup(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LOOKUP) else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    let focused = window.is_focused().unwrap_or(false);
    if visible && focused {
        let _ = window.hide();
    } else {
        show_lookup(app);
    }
}

/// The lookup window lives for the whole session, hidden until the
/// shortcut is pressed. It is created after the app state is managed, so
/// its first commands always find it.
fn create_lookup(app: &AppHandle) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, LOOKUP, WebviewUrl::App("lookup.html".into()))
        .title("Quick Dictionary")
        .inner_size(580.0, 560.0)
        .min_inner_size(380.0, 220.0)
        .decorations(false)
        .visible(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .center()
        .shadow(true)
        .build()?;
    Ok(())
}

fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("settings.html".into()))
        .title(text(ui_lang(app), Text::SettingsTitle))
        .inner_size(680.0, 620.0)
        .min_inner_size(520.0, 400.0)
        .center()
        .build();
    if let Err(e) = built {
        eprintln!("opening settings: {e}");
    }
}

// --- Settings that act on the system ------------------------------------

/// Replaces the registered global shortcut. On failure (an invalid or
/// already-taken shortcut), the previous one stays registered.
fn apply_shortcut(app: &AppHandle, old: Option<&str>, new: &str, lang: Lang) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    if let Some(old) = old {
        let _ = shortcuts.unregister(old);
    }
    shortcuts.register(new).map_err(|e| {
        if let Some(old) = old {
            let _ = shortcuts.register(old);
        }
        i18n::shortcut_error(lang, new, e)
    })
}

fn apply_autostart(app: &AppHandle, enabled: bool, lang: Lang) -> Result<(), String> {
    let autostart = app.autolaunch();
    let result = match (enabled, autostart.is_enabled().unwrap_or(false)) {
        (true, false) => autostart.enable(),
        (false, true) => autostart.disable(),
        _ => Ok(()),
    };
    result.map_err(|e| i18n::autostart_error(lang, e))
}

// --- Commands called by the windows --------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    /// True while packs are still opening: results may be incomplete.
    loading: bool,
    /// No pack is installed (and none is loading).
    no_packs: bool,
    results: Vec<SearchResult>,
    completions: Vec<Suggestion>,
}

#[tauri::command]
fn search(query: String, state: State<AppState>) -> Result<SearchResponse, String> {
    let library = &state.library;
    let limit = state.settings.lock().unwrap().max_results;
    let results = library.search(&query, limit)?;
    let completions = library.complete(&query, 8);
    let loading = library.is_loading();
    let no_packs = !loading && library.pack_count() == 0;
    Ok(SearchResponse {
        loading,
        no_packs,
        results,
        completions,
    })
}

#[tauri::command]
fn hide_lookup(app: AppHandle) {
    if let Some(window) = app.get_webview_window(LOOKUP) {
        let _ = window.hide();
    }
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    show_settings(&app);
}

/// Errors from the windows' scripts, written to the app's log (stderr).
#[tauri::command]
fn log_error(window: tauri::Window, message: String) {
    eprintln!("[{}] {message}", window.label());
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

/// The interface language code ("en", "fr"), with "same as the system"
/// resolved.
#[tauri::command]
fn ui_language(app: AppHandle) -> &'static str {
    ui_lang(&app).code()
}

/// Saves new settings and applies them. Returns the settings now in effect:
/// if the shortcut could not be registered, the old one is kept.
#[tauri::command]
fn save_settings(
    new: Settings,
    app: AppHandle,
    state: State<AppState>,
) -> Result<Settings, String> {
    let old = state.settings.lock().unwrap().clone();
    let mut new = new;
    new.max_results = new.max_results.clamp(1, 50);
    // Messages use the language being switched to.
    let lang = Lang::resolve(new.language);
    let mut errors = Vec::new();
    if new.shortcut != old.shortcut
        && let Err(e) = apply_shortcut(&app, Some(&old.shortcut), &new.shortcut, lang)
    {
        errors.push(e);
        new.shortcut = old.shortcut.clone();
    }
    if new.start_with_system != old.start_with_system
        && let Err(e) = apply_autostart(&app, new.start_with_system, lang)
    {
        errors.push(e);
        new.start_with_system = old.start_with_system;
    }
    if let Err(e) = new.save(&state.settings_file) {
        errors.push(i18n::save_error(lang, e));
    }
    *state.settings.lock().unwrap() = new.clone();
    if new.language != old.language {
        apply_language(&app);
    }
    let _ = app.emit("settings-changed", &new);
    if errors.is_empty() {
        Ok(new)
    } else {
        Err(errors.join("\n"))
    }
}

#[tauri::command]
fn library_info(state: State<AppState>) -> LibraryInfo {
    state.library.info()
}

/// Asks for a pack file and installs it. Returns its language code, or
/// None if the user cancelled.
#[tauri::command]
async fn add_pack(app: AppHandle) -> Result<Option<String>, String> {
    let lang = ui_lang(&app);
    let picked = app
        .dialog()
        .file()
        .set_title(text(lang, Text::AddPackTitle))
        .add_filter(text(lang, Text::PackFilter), &["db"])
        .blocking_pick_file();
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked.into_path().map_err(|e| e.to_string())?;
    let pack_lang = app.state::<AppState>().library.install(&path, lang)?;
    let _ = app.emit("library-changed", ());
    Ok(Some(pack_lang))
}

/// Asks for confirmation, then deletes an installed pack. Returns false if
/// the user cancelled.
#[tauri::command]
async fn remove_pack(file: String, app: AppHandle) -> Result<bool, String> {
    let lang = ui_lang(&app);
    let confirmed = app
        .dialog()
        .message(i18n::remove_question(lang, &file))
        .title(text(lang, Text::RemoveTitle))
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            text(lang, Text::RemoveButton).into(),
            text(lang, Text::Cancel).into(),
        ))
        .blocking_show();
    if !confirmed {
        return Ok(false);
    }
    app.state::<AppState>().library.remove(&file, lang)?;
    let _ = app.emit("library-changed", ());
    Ok(true)
}

/// The packs published for download.
#[tauri::command]
async fn available_packs(app: AppHandle) -> Result<Vec<downloads::RemotePack>, String> {
    downloads::available(&downloads::base_url(), ui_lang(&app))
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    lang: String,
    received: u64,
    total: u64,
}

/// Downloads a published pack and installs it, replacing an older version.
/// Sends "download-progress" events while downloading.
#[tauri::command]
async fn download_pack(lang: String, app: AppHandle) -> Result<(), String> {
    let ui = ui_lang(&app);
    let state = app.state::<AppState>();
    if !state.downloading.lock().unwrap().insert(lang.clone()) {
        return Ok(()); // already on its way
    }
    let result = (|| {
        let base = downloads::base_url();
        let pack = downloads::available(&base, ui)?
            .into_iter()
            .find(|p| p.lang == lang)
            .ok_or_else(|| i18n::manifest_error(ui, format!("no {lang} pack")))?;
        let dest = state.library.download_path(&lang)?;
        let (events, event_lang, total) = (app.clone(), lang.clone(), pack.size);
        let mut last_percent = None;
        downloads::download(&base, &pack, &dest, ui, move |received| {
            // One event per percent is plenty for a progress bar.
            let percent = received * 100 / total.max(1);
            if last_percent != Some(percent) {
                last_percent = Some(percent);
                let progress = DownloadProgress {
                    lang: event_lang.clone(),
                    received,
                    total,
                };
                let _ = events.emit("download-progress", progress);
            }
        })?;
        state.library.install_prepared(&dest, ui)
    })();
    state.downloading.lock().unwrap().remove(&lang);
    let _ = app.emit("library-changed", ());
    result.map(|_| ())
}

#[tauri::command]
async fn reload_packs(app: AppHandle) -> Result<(), String> {
    let result = app.state::<AppState>().library.reload();
    let _ = app.emit("library-changed", ());
    result
}

#[tauri::command]
fn open_packs_folder(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let dir = &state.library.packs_dir;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Opens a web link in the default browser (links in the settings window).
#[tauri::command]
fn open_url(url: String, app: AppHandle) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err(format!("not a web link: {url}"));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

// --- Setup ---------------------------------------------------------------

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let lang = ui_lang(app);
    let item =
        |id: &str, label: Text| MenuItem::with_id(app, id, text(lang, label), true, None::<&str>);
    let lookup = item("lookup", Text::TrayLookup)?;
    let settings = item("settings", Text::TraySettings)?;
    let quit = item("quit", Text::TrayQuit)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&lookup, &settings, &separator, &quit])?;
    *app.state::<AppState>().tray_items.lock().unwrap() = vec![
        (lookup, Text::TrayLookup),
        (settings, Text::TraySettings),
        (quit, Text::TrayQuit),
    ];
    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("bundled icon"))
        .tooltip("Quick Dictionary")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "lookup" => show_lookup(app),
            "settings" => show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_lookup(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        // Must come first: a second launch just shows the running app.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_lookup(app)
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        toggle_lookup(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let paths = app.path();
            // QD_PACKS points the app at another packs folder (development).
            let packs_dir = std::env::var_os("QD_PACKS")
                .map(PathBuf::from)
                .unwrap_or(paths.app_data_dir()?.join("packs"));
            let cache_dir = paths.app_cache_dir()?.join("indexes");
            let settings_file = settings_file(&paths.app_config_dir()?);
            let settings = Settings::load(&settings_file);

            let handle = app.handle().clone();
            let lang = Lang::resolve(settings.language);
            if let Err(e) = apply_shortcut(&handle, None, &settings.shortcut, lang) {
                eprintln!("{e}");
            }
            // Development builds would register themselves to start at login:
            // they only do so when the setting is changed explicitly.
            if !cfg!(debug_assertions)
                && let Err(e) = apply_autostart(&handle, settings.start_with_system, lang)
            {
                eprintln!("{e}");
            }
            app.manage(AppState {
                library: Library::new(packs_dir, cache_dir),
                settings: Mutex::new(settings),
                settings_file,
                tray_items: Mutex::new(Vec::new()),
                downloading: Mutex::new(HashSet::new()),
            });
            create_lookup(&handle)?;
            setup_tray(&handle)?;

            // Open the packs in the background: the first time, building
            // their search indexes takes a few seconds.
            std::thread::spawn(move || {
                let state = handle.state::<AppState>();
                if let Err(e) = state.library.reload() {
                    eprintln!("opening packs: {e}");
                }
                let _ = handle.emit("library-changed", ());
                if state.library.pack_count() == 0 {
                    let ui = handle.clone();
                    let _ = handle.run_on_main_thread(move || show_settings(&ui));
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // The lookup window lives for the whole session: closing hides it.
            WindowEvent::CloseRequested { api, .. } if window.label() == LOOKUP => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Focused(false)
                if window.label() == LOOKUP && settings(window.app_handle()).hide_on_blur =>
            {
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            search,
            hide_lookup,
            open_settings,
            get_settings,
            ui_language,
            save_settings,
            library_info,
            add_pack,
            remove_pack,
            reload_packs,
            open_packs_folder,
            open_url,
            log_error,
            available_packs,
            download_pack,
        ])
        .build(tauri::generate_context!())
        .expect("building the app")
        // Keep running in the tray when every window is closed.
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event
                && code.is_none()
            {
                api.prevent_exit();
            }
        });
}

#[cfg(test)]
mod tests {
    /// The release workflow names releases after the Cargo.toml version:
    /// the app must report that same version (tauri.conf.json sets none).
    #[test]
    fn app_version_comes_from_cargo_toml() {
        let context: tauri::Context<tauri::Wry> = tauri::generate_context!();
        assert_eq!(
            context.package_info().version.to_string(),
            env!("CARGO_PKG_VERSION")
        );
    }
}
