mod auth;
mod config;
mod db;
mod discord;
mod launcher;
mod security;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(discord::EstadoDiscord(std::sync::Mutex::new(None)))
        .setup(|app| {
            db::init(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            auth::microsoft::auth_microsoft,
            auth::azauth::auth_azauth,
            auth::azauth::auth_azauth_2fa,
            auth::offline::auth_offline,
            auth::refresh_token,
            auth::logout,
            config::get_launcher_config,
            db::get_config,
            db::set_config,
            db::get_accounts,
            db::save_account_cmd,
            db::delete_account_cmd,
            security::verify_access_code,
            launcher::instance::get_instances,
            launcher::downloader::download_instance,
            launcher::downloader::cancel_download,
            launcher::java::get_java_paths,
            launcher::java::get_best_java_for_version,
            launcher::process::launch_game,
            launcher::process::get_hidden_mods_dir,
            discord::init_discord_rpc,
            discord::update_rpc,
            discord::stop_rpc,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
