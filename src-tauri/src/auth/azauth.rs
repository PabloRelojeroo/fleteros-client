use super::{Account, AuthType};
use serde::Deserialize;
use tauri::AppHandle;

#[derive(Deserialize)]
struct RespuestaAZauth {
    #[serde(rename = "access_token")]
    access_token: Option<String>,
    uuid: Option<String>,
    username: Option<String>,
    #[serde(rename = "A2F")]
    a2f: Option<bool>,
    #[serde(rename = "session")]
    session: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct RespuestaRefrescoAZauth {
    access_token: Option<String>,
    error: Option<String>,
}

#[tauri::command]
pub async fn auth_azauth(
    server_url: String,
    username: String,
    password: String,
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cliente = reqwest::Client::new();
    let cuerpo = serde_json::json!({
        "username": username,
        "password": password,
    });

    let respuesta: RespuestaAZauth = cliente
        .post(format!("{server_url}/authenticate"))
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| format!("AZauth parse error: {e}"))?;

    if let Some(err) = respuesta.error {
        return Err(err);
    }

    if respuesta.a2f == Some(true) {
        let sesion = respuesta.session.ok_or("No 2FA session token")?;
        return Ok(serde_json::json!({
            "requires2FA": true,
            "session": sesion,
            "serverUrl": server_url,
            "username": username,
        }));
    }

    let cuenta = construir_cuenta_desde_azauth(respuesta, server_url)?;
    crate::db::save_account(&app, &cuenta)
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::to_value(&cuenta).unwrap())
}

#[tauri::command]
pub async fn auth_azauth_2fa(
    server_url: String,
    session: String,
    code: String,
    _username: String,
    app: AppHandle,
) -> Result<Account, String> {
    let cliente = reqwest::Client::new();
    let cuerpo = serde_json::json!({
        "session": session,
        "code": code,
    });

    let respuesta: RespuestaAZauth = cliente
        .post(format!("{server_url}/authenticate/2fa"))
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| format!("AZauth 2FA parse error: {e}"))?;

    if let Some(err) = respuesta.error {
        return Err(err);
    }

    let cuenta = construir_cuenta_desde_azauth(respuesta, server_url)?;
    crate::db::save_account(&app, &cuenta)
        .await
        .map_err(|e| e.to_string())?;

    Ok(cuenta)
}

fn construir_cuenta_desde_azauth(respuesta: RespuestaAZauth, server_url: String) -> Result<Account, String> {
    let access_token = respuesta.access_token.ok_or("No access token in response")?;
    let uuid = respuesta.uuid.ok_or("No UUID in response")?;
    let username = respuesta.username.ok_or("No username in response")?;

    Ok(Account {
        id: uuid.clone(),
        username,
        uuid,
        access_token,
        refresh_token: Some(server_url),
        auth_type: AuthType::AZauth,
        skin_url: None,
    })
}

pub async fn refrescar_token_azauth(cuenta: Account, app: &AppHandle) -> Result<Account, String> {
    let server_url = cuenta
        .refresh_token
        .as_deref()
        .ok_or("No server URL stored for AZauth refresh")?;

    let cliente = reqwest::Client::new();
    let cuerpo = serde_json::json!({
        "access_token": cuenta.access_token,
    });

    let respuesta: RespuestaRefrescoAZauth = cliente
        .post(format!("{server_url}/refresh"))
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| format!("AZauth refresh parse error: {e}"))?;

    if let Some(err) = respuesta.error {
        return Err(err);
    }

    let token_nuevo = respuesta.access_token.ok_or("No access token in refresh response")?;
    let actualizada = Account {
        access_token: token_nuevo,
        ..cuenta
    };

    crate::db::save_account(app, &actualizada)
        .await
        .map_err(|e| e.to_string())?;

    Ok(actualizada)
}
