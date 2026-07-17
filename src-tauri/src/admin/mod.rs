use serde::Serialize;

#[derive(Serialize)]
struct Identidad {
    uuid: String,
    username: String,
}

fn admin_api_url(base_url: &str) -> String {
    format!("{}/files/admin_api.php", base_url.trim_end_matches('/'))
}

async fn llamar(
    base_url: &str,
    cuerpo: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let cliente = reqwest::Client::new();
    let respuesta: serde_json::Value = cliente
        .post(admin_api_url(base_url))
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| format!("Admin API parse error: {e}"))?;

    if respuesta.get("success").and_then(|v| v.as_bool()) == Some(true) {
        Ok(respuesta)
    } else {
        Err(respuesta
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Error desconocido del panel admin")
            .to_string())
    }
}

#[tauri::command]
pub async fn admin_check_access(
    base_url: String,
    uuid: String,
    username: String,
) -> Result<serde_json::Value, String> {
    llamar(
        &base_url,
        serde_json::json!({
            "action": "check_access",
            "identity": Identidad { uuid, username },
        }),
    )
    .await
}

#[tauri::command]
pub async fn admin_get_instances(
    base_url: String,
    uuid: String,
    username: String,
) -> Result<serde_json::Value, String> {
    let respuesta = llamar(
        &base_url,
        serde_json::json!({
            "action": "get_instances",
            "identity": Identidad { uuid, username },
        }),
    )
    .await?;
    Ok(respuesta.get("instances").cloned().unwrap_or_default())
}

#[tauri::command]
pub async fn admin_save_instance(
    base_url: String,
    uuid: String,
    username: String,
    old_name: String,
    instance: serde_json::Value,
) -> Result<String, String> {
    let respuesta = llamar(
        &base_url,
        serde_json::json!({
            "action": "save_instance",
            "oldName": old_name,
            "instance": instance,
            "identity": Identidad { uuid, username },
        }),
    )
    .await?;
    Ok(respuesta
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string())
}

#[tauri::command]
pub async fn admin_delete_instance(
    base_url: String,
    uuid: String,
    username: String,
    name: String,
) -> Result<(), String> {
    llamar(
        &base_url,
        serde_json::json!({
            "action": "delete_instance",
            "name": name,
            "identity": Identidad { uuid, username },
        }),
    )
    .await?;
    Ok(())
}

#[tauri::command]
pub async fn admin_upload_image(
    base_url: String,
    uuid: String,
    username: String,
    instance_name: String,
    kind: String,
    file_path: String,
) -> Result<String, String> {
    let bytes = std::fs::read(&file_path).map_err(|e| format!("No se pudo leer el archivo: {e}"))?;
    if bytes.len() > 5 * 1024 * 1024 {
        return Err("La imagen no puede superar 5MB".to_string());
    }

    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("imagen.png")
        .to_string();

    use base64::Engine;
    let data_base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    let respuesta = llamar(
        &base_url,
        serde_json::json!({
            "action": "upload_image",
            "instanceName": instance_name,
            "kind": kind,
            "filename": filename,
            "dataBase64": data_base64,
            "identity": Identidad { uuid, username },
        }),
    )
    .await?;

    Ok(respuesta
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string())
}

#[tauri::command]
pub async fn admin_list_permissions(
    base_url: String,
    token: String,
) -> Result<serde_json::Value, String> {
    let respuesta = llamar(
        &base_url,
        serde_json::json!({
            "action": "list_permissions",
            "token": token,
        }),
    )
    .await?;
    Ok(respuesta.get("permissions").cloned().unwrap_or_default())
}

#[tauri::command]
pub async fn admin_set_permissions(
    base_url: String,
    token: String,
    permissions: serde_json::Value,
) -> Result<(), String> {
    llamar(
        &base_url,
        serde_json::json!({
            "action": "set_permissions",
            "token": token,
            "permissions": permissions,
        }),
    )
    .await?;
    Ok(())
}

#[tauri::command]
pub async fn admin_login(
    base_url: String,
    username: String,
    password: String,
) -> Result<String, String> {
    let respuesta = llamar(
        &base_url,
        serde_json::json!({
            "action": "login",
            "username": username,
            "password": password,
        }),
    )
    .await?;
    Ok(respuesta
        .get("token")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string())
}
