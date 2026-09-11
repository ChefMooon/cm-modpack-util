// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod changelog;
mod changelog_transport;
pub mod db;
pub mod discovery;
pub mod domain;
pub mod safety;
pub mod watcher;

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
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            let database = db::initialize(&app_data_dir.join("cm-modpack-util.sqlite"))
                .map_err(|error| format!("Failed to initialize application database: {error}"))?;
            let start_minimized = db::bool_setting(&database, "general.startMinimized", false);
            let restore_window_state =
                db::bool_setting(&database, "general.restoreWindowState", true);
            app.manage(database);
            app.manage(discovery::operations::DiscoveryRuntime::default());
            app.manage(changelog::ChangelogRuntime::default());
            app.manage(watcher::ReleaseWatcherCoordinator::default());

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
            db::preview_modpack,
            db::register_modpack,
            db::list_modpacks,
            db::list_snapshots,
            db::get_snapshot,
            db::set_snapshot_decision,
            db::save_snapshot_note,
            db::close_snapshot,
            db::link_snapshot_retry,
            db::recheck_snapshot,
            db::open_modpack,
            db::refresh_modpack,
            db::get_modpack_inventory,
            db::get_modpack_overview,
            db::update_modpack_metadata,
            db::archive_modpack,
            db::restore_modpack,
            db::disconnect_modpack,
            db::reconnect_modpack,
            db::create_release,
            db::get_release,
            db::list_releases_command,
            db::update_release,
            db::compare_releases,
            db::start_release_workspace,
            db::list_release_workspaces,
            db::load_release_workspace,
            db::list_release_workspace_activity,
            db::list_release_workspace_observations,
            db::get_release_workspace_evidence,
            db::abandon_release_workspace,
            db::unlink_snapshot_from_release_workspace,
            db::rebase_release_workspace,
            db::link_snapshot_to_release_workspace,
            db::set_release_workspace_decision,
            db::select_release_workspace_changelog,
            db::create_release_workspace_changelog,
            db::finalize_release_workspace,
            db::publish_release_workspace,
            db::withdraw_release_workspace,
            db::lifecycle::preview_lifecycle_action,
            db::lifecycle::apply_lifecycle_action,
            db::lifecycle::preview_cleanup,
            db::lifecycle::execute_cleanup,
            db::lifecycle::get_storage_report,
            watcher::start_release_workspace_watcher,
            watcher::stop_release_workspace_watcher,
            watcher::reconcile_release_workspace_watchers,
            db::get_operation_history,
            db::record_recovery_acknowledgement,
            discovery::operations::cancel_update_check,
            discovery::pipeline::check_for_updates,
            discovery::mutation::pin_modpack,
            discovery::mutation::unpin_modpack,
            discovery::mutation::cancel_operation,
            discovery::mutation::apply_modpack,
            changelog::generate_changelog,
            changelog::cancel_changelog_generation,
            changelog::create_changelog_revision,
            changelog::create_changelog_selection_revision,
            changelog::archive_changelog_revision,
            changelog::get_changelog_artifact,
            changelog::list_changelog_artifacts,
            changelog::list_changelog_revisions,
            changelog::list_changelog_exports,
            changelog::export_changelog
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
