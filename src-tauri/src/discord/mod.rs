use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct EstadoDiscord(pub Mutex<Option<DiscordIpcClient>>);

#[tauri::command]
pub async fn init_discord_rpc(client_id: String, app: AppHandle) -> Result<(), String> {
    let mut cliente = DiscordIpcClient::new(&client_id).map_err(|e| e.to_string())?;
    cliente.connect().map_err(|e| e.to_string())?;
    *app.state::<EstadoDiscord>().0.lock().map_err(|e| e.to_string())? = Some(cliente);
    Ok(())
}

#[tauri::command]
pub async fn update_rpc(
    details: String,
    state: String,
    app: AppHandle,
) -> Result<(), String> {
    let estado_discord = app.state::<EstadoDiscord>();
    let mut guardia = estado_discord.0.lock().map_err(|e| e.to_string())?;

    if let Some(cliente) = guardia.as_mut() {
        cliente
            .set_activity(
                activity::Activity::new()
                    .details(&details)
                    .state(&state),
            )
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn stop_rpc(app: AppHandle) -> Result<(), String> {
    let estado_discord = app.state::<EstadoDiscord>();
    let mut guardia = estado_discord.0.lock().map_err(|e| e.to_string())?;
    if let Some(mut cliente) = guardia.take() {
        cliente.close().ok();
    }
    Ok(())
}
