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

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

use config::Config;
use soundpack::{PackInfo, SoundPack};
use state::AppShared;

/// Live tray menu-item handles, kept so their state can follow config changes.
struct TrayHandles {
    enabled: CheckMenuItem<tauri::Wry>,
    mouse: CheckMenuItem<tauri::Wry>,
    /// (pack id, menu item) pairs acting as a radio group.
    packs: Vec<(String, CheckMenuItem<tauri::Wry>)>,
}

/// Push a config snapshot to both the tray checkboxes and the settings UI.
fn broadcast(app: &AppHandle, cfg: &Config) {
    if let Some(tray) = app.try_state::<Mutex<TrayHandles>>() {
        let tray = tray.lock().unwrap();
        let _ = tray.enabled.set_checked(cfg.enabled);
        let _ = tray.mouse.set_checked(cfg.mouse_enabled);
        for (id, item) in &tray.packs {
            let _ = item.set_checked(*id == cfg.pack);
        }
    }
    let _ = app.emit("config", cfg);
}

fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
fn get_config(state: State<Arc<AppShared>>) -> Config {
    state.snapshot()
}

#[tauri::command]
fn list_packs() -> Vec<PackInfo> {
    SoundPack::list()
}

#[tauri::command]
fn set_pack(pack: String, app: AppHandle, state: State<Arc<AppShared>>) -> Config {
    let cfg = state.set_pack(&pack);
    broadcast(&app, &cfg);
    cfg
}

#[tauri::command]
fn set_enabled(enabled: bool, app: AppHandle, state: State<Arc<AppShared>>) -> Config {
    let cfg = state.update(|c| c.enabled = enabled);
    broadcast(&app, &cfg);
    cfg
}

#[tauri::command]
fn set_mouse_enabled(enabled: bool, app: AppHandle, state: State<Arc<AppShared>>) -> Config {
    let cfg = state.update(|c| c.mouse_enabled = enabled);
    broadcast(&app, &cfg);
    cfg
}

#[tauri::command]
fn set_volume(volume: f32, app: AppHandle, state: State<Arc<AppShared>>) -> Config {
    let cfg = state.update(|c| c.volume = volume);
    broadcast(&app, &cfg);
    cfg
}

#[tauri::command]
fn set_pitch(pitch: f32, app: AppHandle, state: State<Arc<AppShared>>) -> Config {
    let cfg = state.update(|c| c.pitch_variation = pitch);
    broadcast(&app, &cfg);
    cfg
}

#[tauri::command]
fn set_play_on_release(play: bool, app: AppHandle, state: State<Arc<AppShared>>) -> Config {
    let cfg = state.update(|c| c.play_on_release = play);
    broadcast(&app, &cfg);
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

fn build_tray(app: &mut tauri::App, cfg: &Config) -> tauri::Result<()> {
    let enabled_item = CheckMenuItem::with_id(
        app,
        "toggle_enabled",
        "Keyboard Sounds",
        true,
        cfg.enabled,
        None::<&str>,
    )?;
    let mouse_item = CheckMenuItem::with_id(
        app,
        "toggle_mouse",
        "Mouse Sounds",
        true,
        cfg.mouse_enabled,
        None::<&str>,
    )?;

    let mut pack_items = Vec::new();
    for info in SoundPack::list() {
        let item = CheckMenuItem::with_id(
            app,
            format!("pack:{}", info.id),
            &info.name,
            true,
            info.id == cfg.pack,
            None::<&str>,
        )?;
        pack_items.push((info.id, item));
    }
    let pack_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = pack_items
        .iter()
        .map(|(_, item)| item as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
        .collect();
    let pack_menu = Submenu::with_items(app, "Sound Pack", true, &pack_refs)?;

    let settings_item = MenuItem::with_id(app, "open_settings", "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Thock", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &enabled_item,
            &mouse_item,
            &PredefinedMenuItem::separator(app)?,
            &pack_menu,
            &PredefinedMenuItem::separator(app)?,
            &settings_item,
            &quit_item,
        ],
    )?;

    app.manage(Mutex::new(TrayHandles {
        enabled: enabled_item,
        mouse: mouse_item,
        packs: pack_items,
    }));

    TrayIconBuilder::with_id("thock-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Thock")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let shared = app.state::<Arc<AppShared>>();
            match event.id.as_ref() {
                "toggle_enabled" => {
                    let next = !shared.snapshot().enabled;
                    let cfg = shared.update(|c| c.enabled = next);
                    broadcast(app, &cfg);
                }
                "toggle_mouse" => {
                    let next = !shared.snapshot().mouse_enabled;
                    let cfg = shared.update(|c| c.mouse_enabled = next);
                    broadcast(app, &cfg);
                }
                "open_settings" => show_settings(app),
                "quit" => app.exit(0),
                id => {
                    if let Some(pack_id) = id.strip_prefix("pack:") {
                        let cfg = shared.set_pack(pack_id);
                        shared.play_test();
                        broadcast(app, &cfg);
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init());

    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    builder
        .invoke_handler(tauri::generate_handler![
            get_config,
            list_packs,
            set_pack,
            set_enabled,
            set_mouse_enabled,
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
            let pack = SoundPack::load(&cfg.pack);
            let audio_tx = audio::spawn();

            let shared = Arc::new(AppShared::new(cfg.clone(), pack, audio_tx, config_path));
            app.manage(shared.clone());

            listener::spawn(shared);
            build_tray(app, &cfg)?;

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
