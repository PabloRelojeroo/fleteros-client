use super::{Account, AuthType};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

const ID_CLIENTE: &str = "13f589e1-e2fc-443e-a68a-63b0092b8eeb";
const URL_REDIRECCION: &str = "https://login.live.com/oauth20_desktop.srf";

#[derive(Deserialize)]
struct RespuestaTokenMsa {
    access_token: String,
    refresh_token: String,
}

#[derive(Deserialize)]
struct RespuestaXbl {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: ReclamosXbl,
}

#[derive(Deserialize)]
struct ReclamosXbl {
    xui: Vec<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct RespuestaXsts {
    #[serde(rename = "Token")]
    token: String,
}

#[derive(Deserialize)]
struct RespuestaTokenMc {
    access_token: String,
}

#[derive(Deserialize)]
struct RespuestaPerfilMc {
    id: String,
    name: String,
    skins: Option<Vec<SkinMc>>,
}

#[derive(Deserialize)]
struct SkinMc {
    url: String,
    state: String,
}

#[tauri::command]
pub async fn auth_microsoft(app: AppHandle) -> Result<Account, String> {
    let url_auth_str = format!(
        "https://login.live.com/oauth20_authorize.srf\
         ?client_id={ID_CLIENTE}\
         &response_type=code\
         &scope=XboxLive.signin%20offline_access\
         &redirect_uri=https%3A%2F%2Flogin.live.com%2Foauth20_desktop.srf\
         &prompt=select_account"
    );

    let url_auth: tauri::Url = url_auth_str
        .parse()
        .map_err(|e| format!("URL parse error: {e}"))?;

    let (emisor, receptor) = tokio::sync::oneshot::channel::<Result<String, String>>();
    let emisor_compartido = Arc::new(Mutex::new(Some(emisor)));
    let emisor_nav = emisor_compartido.clone();

    let _ventana = WebviewWindowBuilder::new(&app, "ms-auth", WebviewUrl::External(url_auth))
        .title("Iniciar sesión con Microsoft")
        .inner_size(480.0, 680.0)
        .center()
        .on_navigation(move |url| {
            if url.as_str().starts_with(URL_REDIRECCION) {
                let resultado = url
                    .query_pairs()
                    .find(|(clave, _)| clave == "code")
                    .map(|(_, valor)| valor.into_owned())
                    .ok_or_else(|| "Código no encontrado en redirect".to_string());
                if let Ok(mut guard) = emisor_nav.lock() {
                    if let Some(sender) = guard.take() {
                        let _ = sender.send(resultado);
                    }
                }
                false
            } else {
                true
            }
        })
        .build()
        .map_err(|e| e.to_string())?;

    let codigo = receptor.await.map_err(|_| "Autenticación cancelada".to_string())??;

    if let Some(w) = app.get_webview_window("ms-auth") {
        let _ = w.close();
    }

    let token_msa = intercambiar_codigo_por_token(&codigo).await?;
    let token_xbl = obtener_token_xbox_live(&token_msa.access_token).await?;
    let uhs = extraer_uhs(&token_xbl)?;
    let token_xsts = obtener_token_xsts(&token_xbl.token).await?;
    let token_mc = obtener_token_minecraft(&token_xsts.token, &uhs).await?;
    let perfil = obtener_perfil_minecraft(&token_mc.access_token).await?;

    let url_skin = perfil
        .skins
        .as_ref()
        .and_then(|skins| skins.iter().find(|s| s.state == "ACTIVE"))
        .map(|s| s.url.clone());

    let cuenta = Account {
        id: perfil.id.clone(),
        username: perfil.name,
        uuid: formatear_uuid(&perfil.id),
        access_token: token_mc.access_token,
        refresh_token: Some(token_msa.refresh_token),
        auth_type: AuthType::Microsoft,
        skin_url: url_skin,
    };

    crate::db::save_account(&app, &cuenta)
        .await
        .map_err(|e| e.to_string())?;

    Ok(cuenta)
}

async fn intercambiar_codigo_por_token(codigo: &str) -> Result<RespuestaTokenMsa, String> {
    let cliente = reqwest::Client::new();
    let parametros = [
        ("client_id", ID_CLIENTE),
        ("code", codigo),
        ("grant_type", "authorization_code"),
        ("redirect_uri", URL_REDIRECCION),
    ];

    cliente
        .post("https://login.live.com/oauth20_token.srf")
        .form(&parametros)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<RespuestaTokenMsa>()
        .await
        .map_err(|e| format!("MSA token parse error: {e}"))
}

async fn obtener_token_xbox_live(token_acceso_msa: &str) -> Result<RespuestaXbl, String> {
    let cuerpo = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={token_acceso_msa}")
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    reqwest::Client::new()
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<RespuestaXbl>()
        .await
        .map_err(|e| format!("XBL parse error: {e}"))
}

fn extraer_uhs(xbl: &RespuestaXbl) -> Result<String, String> {
    xbl.display_claims
        .xui
        .first()
        .and_then(|xui| xui.get("uhs"))
        .cloned()
        .ok_or("UHS not found in XBL response".to_string())
}

async fn obtener_token_xsts(token_xbl: &str) -> Result<RespuestaXsts, String> {
    let cuerpo = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [token_xbl]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    reqwest::Client::new()
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<RespuestaXsts>()
        .await
        .map_err(|e| format!("XSTS parse error: {e}"))
}

async fn obtener_token_minecraft(token_xsts: &str, uhs: &str) -> Result<RespuestaTokenMc, String> {
    let cuerpo = serde_json::json!({
        "identityToken": format!("XBL3.0 x={uhs};{token_xsts}")
    });

    reqwest::Client::new()
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .header("Content-Type", "application/json")
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<RespuestaTokenMc>()
        .await
        .map_err(|e| format!("MC token parse error: {e}"))
}

async fn obtener_perfil_minecraft(token_mc: &str) -> Result<RespuestaPerfilMc, String> {
    reqwest::Client::new()
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(token_mc)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<RespuestaPerfilMc>()
        .await
        .map_err(|e| format!("MC profile parse error: {e}"))
}

pub async fn refrescar_token_microsoft(cuenta: Account, app: &AppHandle) -> Result<Account, String> {
    let token_refresco = cuenta
        .refresh_token
        .as_deref()
        .ok_or("No refresh token available")?;

    let cliente = reqwest::Client::new();
    let parametros = [
        ("client_id", ID_CLIENTE),
        ("refresh_token", token_refresco),
        ("grant_type", "refresh_token"),
    ];

    let token_msa: RespuestaTokenMsa = cliente
        .post("https://login.live.com/oauth20_token.srf")
        .form(&parametros)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| format!("Refresh token parse error: {e}"))?;

    let token_xbl = obtener_token_xbox_live(&token_msa.access_token).await?;
    let uhs = extraer_uhs(&token_xbl)?;
    let token_xsts = obtener_token_xsts(&token_xbl.token).await?;
    let token_mc = obtener_token_minecraft(&token_xsts.token, &uhs).await?;

    let actualizada = Account {
        access_token: token_mc.access_token,
        refresh_token: Some(token_msa.refresh_token),
        ..cuenta
    };

    crate::db::save_account(app, &actualizada)
        .await
        .map_err(|e| e.to_string())?;

    Ok(actualizada)
}

fn formatear_uuid(crudo: &str) -> String {
    if crudo.len() == 32 {
        format!(
            "{}-{}-{}-{}-{}",
            &crudo[0..8],
            &crudo[8..12],
            &crudo[12..16],
            &crudo[16..20],
            &crudo[20..32]
        )
    } else {
        crudo.to_string()
    }
}
