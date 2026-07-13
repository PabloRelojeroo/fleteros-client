use super::{Account, AuthType};
use tauri::AppHandle;
use uuid::Uuid;

const ESPACIO_NOMBRES_OFFLINE: Uuid = Uuid::from_bytes([
    0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1,
    0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
]);

#[tauri::command]
pub async fn auth_offline(username: String, app: AppHandle) -> Result<Account, String> {
    if username.is_empty() || username.len() > 16 {
        return Err("Username must be 1-16 characters".to_string());
    }

    let uuid = Uuid::new_v3(&ESPACIO_NOMBRES_OFFLINE, username.as_bytes()).to_string();
    let id = uuid.replace('-', "");

    let cuenta = Account {
        id: id.clone(),
        username,
        uuid,
        access_token: "offline".to_string(),
        refresh_token: None,
        auth_type: AuthType::Offline,
        skin_url: None,
    };

    crate::db::save_account(&app, &cuenta)
        .await
        .map_err(|e| e.to_string())?;

    Ok(cuenta)
}
