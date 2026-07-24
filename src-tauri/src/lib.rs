//! Thock — cross-platform mechanical keyboard sounds.
//!
//! An original, open-source utility: it listens for global key events and plays
//! short click samples, with a tray menu and a small settings window.

mod audio;
mod config;
#[cfg(not(target_os = "macos"))]
mod keymap;
mod listener;
mod soundpack;
mod state;

use std::sync::{Arc, Mutex};

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

use config::Config;
use soundpack::SoundPack;
use state::AppShared;

/// Holds live tray menu-item handles so their state can be synced with config.
struct TrayHandles {
    enabled: Mutex<Option<CheckMenuItem<tauri::Wry>>>,
}

fn emit_config(app: &AppHandle, cfg: &Config) {
    let _ = app.emit("config", cfg);
}

fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Apply an enabled/disabled change to config, the tray checkbox, and the UI.
fn set_enabled_everywhere(app: &AppHandle, shared: &AppShared, enabled: bool) -> Config {
    let cfg = shared.update(|c| c.enabled = enabled);
    if let Some(item) = app.state::<TrayHandles>().enabled.lock().unwrap().as_ref() {
        let _ = item.set_checked(enabled);
    }
    emit_config(app, &cfg);
    cfg
}

#[tauri::command]
fn get_config(state: State<Arc<AppShared>>) -> Config {
    state.snapshot()
}

#[tauri::command]
fn set_enabled(enabled: bool, app: AppHandle, state: State<Arc<AppShared>>) -> Config {
    set_enabled_everywhere(&app, state.inner(), enabled)
}

#[tauri::command]
fn set_volume(volume: f32, app: AppHandle, state: State<Arc<AppShared>>) -> Config {
    let cfg = state.update(|c| c.volume = volume);
    emit_config(&app, &cfg);
    cfg
}

#[tauri::command]
fn set_pitch(pitch: f32, app: AppHandle, state: State<Arc<AppShared>>) -> Config {
    let cfg = state.update(|c| c.pitch_variation = pitch);
    emit_config(&app, &cfg);
    cfg
}

#[tauri::command]
fn set_play_on_release(play: bool, app: AppHandle, state: State<Arc<AppShared>>) -> Config {
    let cfg = state.update(|c| c.play_on_release = play);
    emit_config(&app, &cfg);
    cfg
}

#[tauri::command]
fn play_test(state: State<Arc<AppShared>>) {
    state.play_test();
}

#[tauri::command]
fn get_pack_name(state: State<Arc<AppShared>>) -> String {
    state.pack_name()
}

fn build_tray(app: &mut tauri::App, enabled: bool) -> tauri::Result<()> {
    let enabled_item = CheckMenuItem::with_id(
        app,
        "toggle_enabled",
        "Enabled",
        true,
        enabled,
        None::<&str>,
    )?;
    let settings_item = MenuItem::with_id(app, "open_settings", "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Thock", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&enabled_item, &separator, &settings_item, &quit_item],
    )?;

    app.manage(TrayHandles {
        enabled: Mutex::new(Some(enabled_item)),
    });

    TrayIconBuilder::with_id("thock-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Thock")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle_enabled" => {
                let shared = app.state::<Arc<AppShared>>();
                let next = !shared.snapshot().enabled;
                set_enabled_everywhere(app, shared.inner(), next);
            }
            "open_settings" => show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_enabled,
            set_volume,
            set_pitch,
            set_play_on_release,
            play_test,
            get_pack_name,
        ])
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let config_path = config::config_file(&config_dir);
            let cfg = Config::load(&config_path);
            let pack = SoundPack::load_default();
            let audio_tx = audio::spawn();

            let shared = Arc::new(AppShared::new(cfg.clone(), pack, audio_tx, config_path));
            app.manage(shared.clone());

            listener::spawn(shared);
            build_tray(app, cfg.enabled)?;

            // Behave like a menu-bar accessory on macOS (no Dock icon).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the settings window hides it instead of quitting the app.
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
