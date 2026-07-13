use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instancia {
    pub name: String,
    pub url: Option<String>,
    #[serde(rename(deserialize = "loadder"))]
    pub loader: Option<ConfiguracionLoader>,
    pub customization: Option<Personalizacion>,
    pub verify: Option<bool>,
    pub ignored: Option<Vec<String>>,
    pub whitelist: Option<Vec<String>>,
    #[serde(rename(deserialize = "whitelistActive"))]
    pub whitelist_active: Option<bool>,
    #[serde(rename(deserialize = "accessCodes"))]
    pub access_codes: Option<Vec<String>>,
    pub status: Option<EstadoServidor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfiguracionLoader {
    pub minecraft_version: String,
    #[serde(rename(deserialize = "loadder_type"))]
    pub loader_type: String,
    #[serde(rename(deserialize = "loadder_version"))]
    pub loader_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Personalizacion {
    pub logo: Option<String>,
    pub background: Option<String>,
    pub name_display: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstadoServidor {
    #[serde(rename(deserialize = "nameServer"))]
    pub name_server: Option<String>,
    pub ip: Option<String>,
    pub port: Option<u16>,
}

#[tauri::command]
pub async fn get_instances(instances_url: String) -> Result<Vec<Instancia>, String> {
    let mapa: HashMap<String, Instancia> = reqwest::get(&instances_url)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| format!("Instances parse error: {e}"))?;

    let mut instancias: Vec<Instancia> = mapa.into_values().collect();
    instancias.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(instancias)
}
