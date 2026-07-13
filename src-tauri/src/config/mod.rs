use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfiguracionLauncher {
    pub maintenance: Option<bool>,
    pub maintenance_message: Option<String>,
    pub online: Option<bool>,
    pub client_id: Option<String>,
    #[serde(rename = "dataDirectory")]
    pub data_directory: Option<String>,
}

#[tauri::command]
pub async fn get_launcher_config(config_url: String) -> Result<ConfiguracionLauncher, String> {
    reqwest::get(&config_url)
        .await
        .map_err(|e| e.to_string())?
        .json::<ConfiguracionLauncher>()
        .await
        .map_err(|e| format!("Config parse error: {e}"))
}
