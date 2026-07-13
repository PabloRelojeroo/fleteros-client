pub mod azauth;
pub mod microsoft;
pub mod offline;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub auth_type: AuthType,
    pub skin_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AuthType {
    Microsoft,
    AZauth,
    Offline,
}

#[tauri::command]
pub async fn logout(account_id: String, app: tauri::AppHandle) -> Result<(), String> {
    crate::db::delete_account_cmd(app, account_id).await
}

#[tauri::command]
pub async fn refresh_token(account_id: String, app: tauri::AppHandle) -> Result<Account, String> {
    let cuentas = crate::db::get_accounts(app.clone()).await.map_err(|e| e.to_string())?;
    let cuenta = cuentas
        .into_iter()
        .find(|a| a.id == account_id)
        .ok_or("Account not found")?;

    match cuenta.auth_type {
        AuthType::Microsoft => microsoft::refrescar_token_microsoft(cuenta, &app).await,
        AuthType::AZauth => azauth::refrescar_token_azauth(cuenta, &app).await,
        AuthType::Offline => Ok(cuenta),
    }
}
