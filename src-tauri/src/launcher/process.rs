use serde::Deserialize;
use std::path::PathBuf;
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Deserialize)]
pub struct ConfiguracionLanzamiento {
    pub java_path: String,
    pub game_dir: String,
    pub version: String,
    pub min_ram_mb: u32,
    pub max_ram_mb: u32,
    pub width: u32,
    pub height: u32,
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub main_class: String,
    pub extra_jvm_args: Option<Vec<String>>,
}

fn marca_launcher(app: &AppHandle) -> String {
    app.config()
        .product_name
        .clone()
        .unwrap_or_else(|| "launcher".to_string())
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

#[tauri::command]
pub async fn launch_game(config: ConfiguracionLanzamiento, app: AppHandle) -> Result<(), String> {
    let ruta_juego = PathBuf::from(&config.game_dir);
    let jar_version = ruta_juego
        .join("versions")
        .join(&config.version)
        .join(format!("{}.jar", config.version));

    let classpath = construir_classpath(&ruta_juego, &config.version, &jar_version);

    let mut argumentos: Vec<String> = Vec::new();

    argumentos.push(format!("-Xms{}m", config.min_ram_mb));
    argumentos.push(format!("-Xmx{}m", config.max_ram_mb));
    argumentos.push(format!(
        "-Djava.library.path={}/versions/{}/natives",
        config.game_dir, config.version
    ));
    argumentos.push("-Dfile.encoding=UTF-8".to_string());
    argumentos.push(format!(
        "-Dminecraft.launcher.brand={}",
        marca_launcher(&app)
    ));

    if let Some(ref extra) = config.extra_jvm_args {
        argumentos.extend(extra.clone());
    }

    argumentos.push("-cp".to_string());
    argumentos.push(classpath);
    argumentos.push(config.main_class.clone());

    let id_indice_assets = {
        let ruta_json = ruta_juego
            .join("versions")
            .join(&config.version)
            .join(format!("{}.json", config.version));
        std::fs::read_to_string(&ruta_json)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v["assetIndex"]["id"].as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| config.version.clone())
    };

    argumentos.extend([
        "--username".to_string(), config.username,
        "--version".to_string(), config.version.clone(),
        "--gameDir".to_string(), config.game_dir.clone(),
        "--assetsDir".to_string(), format!("{}/assets", config.game_dir),
        "--assetIndex".to_string(), id_indice_assets.clone(),
        "--uuid".to_string(), config.uuid,
        "--accessToken".to_string(), config.access_token,
        "--userType".to_string(), "msa".to_string(),
        "--versionType".to_string(), "release".to_string(),
        "--width".to_string(), config.width.to_string(),
        "--height".to_string(), config.height.to_string(),
    ]);

    app.emit("game-launching", ()).ok();

    app.emit("game-log", format!("[Launcher] AssetIndex: {id_indice_assets}")).ok();
    app.emit("game-log", format!("[Launcher] MainClass: {}", config.main_class)).ok();

    let mut comando = Command::new(&config.java_path);
    comando.args(&argumentos)
        .current_dir(&config.game_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    comando.creation_flags(0x08000000);

    let mut proceso_hijo = comando
        .spawn()
        .map_err(|e| format!("Failed to launch game: {e}"))?;

    let salida_estandar = proceso_hijo.stdout.take().expect("stdout piped");
    let salida_error = proceso_hijo.stderr.take().expect("stderr piped");

    let app_salida = app.clone();
    tokio::spawn(async move {
        let mut lineas = BufReader::new(salida_estandar).lines();
        while let Ok(Some(linea)) = lineas.next_line().await {
            app_salida.emit("game-log", &linea).ok();
        }
    });

    let app_error = app.clone();
    tokio::spawn(async move {
        let mut lineas = BufReader::new(salida_error).lines();
        while let Ok(Some(linea)) = lineas.next_line().await {
            app_error.emit("game-log", &linea).ok();
        }
    });

    tokio::spawn(async move {
        match proceso_hijo.wait().await {
            Ok(estado) => {
                app.emit("game-exited", estado.code().unwrap_or(-1)).ok();
            }
            Err(e) => {
                app.emit("game-error", e.to_string()).ok();
            }
        }
    });

    Ok(())
}

fn construir_classpath(ruta_juego: &PathBuf, _version: &str, jar_version: &PathBuf) -> String {
    let ruta_librerias = ruta_juego.join("libraries");
    let mut entradas_cp: Vec<String> = Vec::new();

    recolectar_jars(&ruta_librerias, &mut entradas_cp);

    entradas_cp.push(jar_version.to_string_lossy().to_string());

    #[cfg(target_os = "windows")]
    let separador = ";";
    #[cfg(not(target_os = "windows"))]
    let separador = ":";

    entradas_cp.join(separador)
}

#[tauri::command]
pub async fn get_hidden_mods_dir(game_dir: String) -> Result<String, String> {
    let ruta = PathBuf::from(&game_dir).join(".drk");
    tokio::fs::create_dir_all(&ruta).await.map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        let ruta_str = ruta.to_string_lossy().to_string();
        tokio::process::Command::new("attrib")
            .args(["+h", "+s", &ruta_str])
            .output()
            .await
            .ok();
    }

    Ok(ruta.to_string_lossy().to_string())
}

fn recolectar_jars(directorio: &PathBuf, entradas: &mut Vec<String>) {
    let Ok(lectura) = std::fs::read_dir(directorio) else { return };
    for entrada in lectura.flatten() {
        let ruta = entrada.path();
        if ruta.is_dir() {
            recolectar_jars(&ruta, entradas);
        } else if ruta.extension().and_then(|e| e.to_str()) == Some("jar") {
            entradas.push(ruta.to_string_lossy().to_string());
        }
    }
}
