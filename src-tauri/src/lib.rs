// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod changelog;
mod changelog_transport;
pub mod db;
pub mod discovery;
pub mod domain;
pub mod safety;

use std::fs;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tauri_plugin_window_state::{AppHandleExt, StateFlags, WindowExt};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                // The frontend preference decides whether restoration happens.
                .skip_initial_state("main")
                .build(),
        )
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data_dir)?;
            let database = db::initialize(&app_data_dir.join("settings.sqlite"))
                .map_err(|error| format!("Failed to initialize settings database: {error}"))?;
            let start_minimized = db::bool_setting(&database, "general.startMinimized", false);
            let restore_window_state =
                db::bool_setting(&database, "general.restoreWindowState", true);
            app.manage(database);
            app.manage(discovery::operations::DiscoveryRuntime::default());
            app.manage(changelog::ChangelogRuntime::default());

            let show = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("CM Modpack Util")
                .icon(
                    app.default_window_icon()
                        .expect("default window icon is configured")
                        .clone(),
                )
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                // Restore before the initially hidden window is revealed. This avoids
                // a frontend-load flash and keeps startup work native.
                if restore_window_state {
                    let _ = window.restore_state(
                        StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED,
                    );
                }
                let _ = window.show();
                if start_minimized {
                    let _ = window.minimize();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let database = window.app_handle().state::<db::Database>();
                if db::bool_setting(&database, "general.hideToTray", false) {
                    api.prevent_close();
                    // Closing is converted to hiding, so persist the latest state now.
                    let _ = window.app_handle().save_window_state(StateFlags::all());
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            db::get_settings,
            db::set_setting,
            db::delete_setting,
            db::reset_settings,
            db::preview_project,
            db::register_project,
            db::list_projects,
            db::list_snapshots,
            db::get_snapshot,
            db::set_snapshot_decision,
            db::save_snapshot_note,
            db::close_snapshot,
            db::link_snapshot_retry,
            db::recheck_snapshot,
            db::open_project,
            db::refresh_project,
            db::get_project_inventory,
            db::get_project_overview,
            db::update_project_metadata,
            db::archive_project,
            db::restore_project,
            db::disconnect_project,
            db::reconnect_project,
            db::get_operation_history,
            db::record_recovery_acknowledgement,
            discovery::operations::cancel_update_check,
            discovery::pipeline::check_for_updates,
            discovery::mutation::pin_project,
            discovery::mutation::unpin_project,
            discovery::mutation::cancel_operation,
            discovery::mutation::apply_project,
            changelog::generate_changelog,
            changelog::cancel_changelog_generation,
            changelog::create_changelog_revision,
            changelog::get_changelog_artifact,
            changelog::list_changelog_artifacts,
            changelog::list_changelog_revisions,
            changelog::list_changelog_exports,
            changelog::export_changelog
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
