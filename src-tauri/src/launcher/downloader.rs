use futures::stream::{self, StreamExt};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Semaphore;
use tokio::io::AsyncWriteExt;

struct EstadoProgresoGlobal {
    bytes_done: AtomicU64,
    bytes_total: u64,
    files_done: AtomicUsize,
    files_total: usize,
    inicio: Instant,
}

fn nuevo_estado_progreso(bytes_total: u64, files_total: usize) -> Arc<EstadoProgresoGlobal> {
    Arc::new(EstadoProgresoGlobal {
        bytes_done: AtomicU64::new(0),
        bytes_total,
        files_done: AtomicUsize::new(0),
        files_total,
        inicio: Instant::now(),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgresoDescarga {
    pub phase: String,
    pub file: String,
    pub downloaded: u64,
    pub total: u64,
    pub speed_bps: f64,
    pub eta_seconds: f64,
    pub files_done: usize,
    pub files_total: usize,
}

#[derive(Debug, Deserialize)]
struct ManifiestoVersiones {
    versions: Vec<EntradaVersion>,
}

#[derive(Debug, Deserialize)]
struct EntradaVersion {
    id: String,
    url: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct DatosVersion {
    downloads: HashMap<String, InfoDescarga>,
    libraries: Vec<Libreria>,
    #[serde(rename = "assetIndex")]
    asset_index: IndiceAssets,
    #[serde(rename = "mainClass")]
    main_class: String,
    arguments: Option<Argumentos>,
    #[serde(rename = "minecraftArguments")]
    minecraft_arguments: Option<String>,
    #[serde(rename = "javaVersion")]
    java_version: Option<InfoJavaVersion>,
}

#[derive(Debug, Deserialize)]
struct InfoJavaVersion {
    #[serde(rename = "majorVersion")]
    major_version: u32,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct InfoDescarga {
    url: String,
    sha1: Option<String>,
    size: Option<u64>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Libreria {
    downloads: Option<DescargasLibreria>,
    name: String,
    rules: Option<Vec<Regla>>,
}

#[derive(Debug, Deserialize)]
struct DescargasLibreria {
    artifact: Option<ArtefactoDescarga>,
    classifiers: Option<HashMap<String, ArtefactoDescarga>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
struct ArtefactoDescarga {
    url: String,
    path: String,
    sha1: Option<String>,
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct Regla {
    action: String,
    os: Option<ReglaOs>,
}

#[derive(Debug, Deserialize)]
struct ReglaOs {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IndiceAssets {
    id: String,
    url: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Argumentos {
    game: Vec<serde_json::Value>,
    jvm: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct DatosAssets {
    objects: HashMap<String, ObjetoAsset>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ObjetoAsset {
    hash: String,
    size: u64,
}

#[derive(Deserialize)]
struct EntradaLoaderFabric {
    loader: InfoLoaderFabric,
}

#[derive(Deserialize)]
struct InfoLoaderFabric {
    version: String,
}

#[derive(Deserialize)]
struct PerfilFabric {
    #[serde(rename = "mainClass")]
    main_class: String,
    libraries: Vec<LibreriaFabric>,
}

#[derive(Deserialize)]
struct LibreriaFabric {
    name: String,
    url: String,
    sha1: Option<String>,
}

#[derive(Deserialize)]
struct PromocionesForge {
    promos: HashMap<String, String>,
}

#[allow(dead_code)]
#[allow(dead_code)]
#[derive(Deserialize)]
struct ArchivoServidor {
    url: String,
    size: u64,
    hash: String,
    path: String,
}

struct TokenCancelacion(Arc<Mutex<bool>>);

#[tauri::command]
pub async fn cancel_download(app: AppHandle) -> Result<(), String> {
    if let Some(token) = app.try_state::<TokenCancelacion>() {
        *token.0.lock().unwrap() = true;
    }
    Ok(())
}

#[tauri::command]
pub async fn download_instance(
    instance_id: String,
    version: String,
    game_dir: String,
    max_concurrent: usize,
    instance_url: Option<String>,
    loader_type: Option<String>,
    loader_version: Option<String>,
    ignored: Option<Vec<String>>,
    java_path: Option<String>,
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cancelacion = Arc::new(Mutex::new(false));
    app.manage(TokenCancelacion(cancelacion.clone()));

    let ruta_juego = PathBuf::from(&game_dir);
    let ruta_versiones = ruta_juego.join("versions").join(&version);
    let ruta_librerias = ruta_juego.join("libraries");
    let ruta_assets = ruta_juego.join("assets");
    app.emit("game-log", format!("[Launcher] Instancia: {instance_id}")).ok();

    tokio::fs::create_dir_all(&ruta_versiones).await.map_err(|e| e.to_string())?;
    tokio::fs::create_dir_all(&ruta_librerias).await.map_err(|e| e.to_string())?;
    tokio::fs::create_dir_all(&ruta_assets).await.map_err(|e| e.to_string())?;

    emit_progreso(&app, "fetch", "Obteniendo manifiesto de versión...", 0, 0, 0, 1);

    let manifiesto: ManifiestoVersiones = reqwest::get(
        "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json",
    )
    .await
    .map_err(|e| e.to_string())?
    .json()
    .await
    .map_err(|e| e.to_string())?;

    let url_version = manifiesto
        .versions
        .iter()
        .find(|v| v.id == version)
        .map(|v| v.url.clone())
        .ok_or(format!("Versión {version} no encontrada en el manifiesto"))?;

    let datos_version: DatosVersion = reqwest::get(&url_version)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let ruta_json = ruta_versiones.join(format!("{version}.json"));
    let texto_json = reqwest::get(&url_version)
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::write(&ruta_json, &texto_json).await.map_err(|e| e.to_string())?;

    let mut archivos_a_descargar: Vec<(String, PathBuf, Option<String>, Option<u64>)> = Vec::new();

    if let Some(descarga_cliente) = datos_version.downloads.get("client") {
        let ruta_jar = ruta_versiones.join(format!("{version}.jar"));
        archivos_a_descargar.push((descarga_cliente.url.clone(), ruta_jar, descarga_cliente.sha1.clone(), descarga_cliente.size));
    }

    let nombre_os = obtener_nombre_os();
    for libreria in &datos_version.libraries {
        if !verificar_reglas_aplican(&libreria.rules, &nombre_os) {
            continue;
        }
        if let Some(descargas) = &libreria.downloads {
            if let Some(artefacto) = &descargas.artifact {
                let ruta = ruta_librerias.join(&artefacto.path);
                archivos_a_descargar.push((artefacto.url.clone(), ruta, artefacto.sha1.clone(), artefacto.size));
            }
            if let Some(clasificadores) = &descargas.classifiers {
                let clave_nativa = format!("natives-{nombre_os}");
                if let Some(nativo) = clasificadores.get(&clave_nativa) {
                    let ruta = ruta_librerias.join(&nativo.path);
                    archivos_a_descargar.push((nativo.url.clone(), ruta, nativo.sha1.clone(), nativo.size));
                }
            }
        }
    }

    let ruta_indice_assets = ruta_assets
        .join("indexes")
        .join(format!("{}.json", datos_version.asset_index.id));
    tokio::fs::create_dir_all(ruta_indice_assets.parent().unwrap())
        .await
        .map_err(|e| e.to_string())?;

    let texto_indice_assets = reqwest::get(&datos_version.asset_index.url)
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::write(&ruta_indice_assets, &texto_indice_assets)
        .await
        .map_err(|e| e.to_string())?;

    let datos_assets: DatosAssets =
        serde_json::from_str(&texto_indice_assets).map_err(|e| e.to_string())?;
    let ruta_objetos = ruta_assets.join("objects");

    for (_, objeto) in &datos_assets.objects {
        let prefijo = &objeto.hash[..2];
        let ruta_asset = ruta_objetos.join(prefijo).join(&objeto.hash);
        let url = format!(
            "https://resources.download.minecraft.net/{prefijo}/{}",
            objeto.hash
        );
        archivos_a_descargar.push((url, ruta_asset, Some(objeto.hash.clone()), Some(objeto.size)));
    }

    ejecutar_descargas_concurrentes(&archivos_a_descargar, max_concurrent, &cancelacion, &app, "download").await;

    let loader = loader_type.as_deref().unwrap_or("none").to_lowercase();
    let version_loader = loader_version.as_deref().unwrap_or("latest");

    let clase_principal_final = if loader == "fabric" {
        emit_progreso(&app, "loader", "Instalando Fabric...", 0, 0, 0, 1);
        match instalar_fabric(&ruta_juego, &version, version_loader, &app).await {
            Ok(main_class) => main_class,
            Err(e) => {
                app.emit("game-log", format!("[Launcher] Advertencia Fabric: {e}")).ok();
                datos_version.main_class.clone()
            }
        }
    } else if loader == "forge" || loader == "neoforge" {
        emit_progreso(&app, "loader", "Instalando Forge...", 0, 0, 0, 1);
        let java = java_path.as_deref().unwrap_or("java");
        match instalar_forge(&ruta_juego, &version, version_loader, java, &loader, &app).await {
            Ok(main_class) => main_class,
            Err(e) => {
                app.emit("game-log", format!("[Launcher] Advertencia Forge: {e}")).ok();
                datos_version.main_class.clone()
            }
        }
    } else {
        datos_version.main_class.clone()
    };

    let lista_ignorados = ignored.unwrap_or_default();

    if let Some(ref url) = instance_url {
        app.emit("game-log", format!("[Launcher] Descargando archivos desde: {url}")).ok();
        emit_progreso(&app, "instance", "Descargando archivos del servidor...", 0, 0, 0, 1);

        match descargar_archivos_servidor(url, &ruta_juego, &lista_ignorados, max_concurrent, &cancelacion, &app).await {
            Ok(cantidad) => {
                app.emit("game-log", format!("[Launcher] {cantidad} archivos sincronizados")).ok();
            }
            Err(e) => {
                app.emit("game-log", format!("[Launcher] Error archivos servidor: {e}")).ok();
            }
        }
    } else {
        app.emit("game-log", "[Launcher] Sin URL de instancia — sin mods del servidor").ok();
    }

    Ok(serde_json::json!({
        "success": true,
        "mainClass": clase_principal_final,
        "gameArgs": datos_version.minecraft_arguments.unwrap_or_default(),
        "javaMajorVersion": datos_version.java_version.map(|j| j.major_version),
    }))
}

async fn ejecutar_descargas_concurrentes(
    archivos: &[(String, PathBuf, Option<String>, Option<u64>)],
    max_concurrent: usize,
    cancelacion: &Arc<Mutex<bool>>,
    app: &AppHandle,
    fase: &str,
) {
    let concurrencia = max_concurrent.max(1).min(32);

    let pendientes: Vec<(String, PathBuf, Option<String>, u64)> = stream::iter(archivos.iter().cloned())
        .map(|(url, ruta, sha1, size)| async move {
            let necesita = necesita_descarga(&ruta, &sha1).await;
            (necesita, url, ruta, sha1, size.unwrap_or(0))
        })
        .buffer_unordered(concurrencia)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .filter(|(necesita, ..)| *necesita)
        .map(|(_, url, ruta, sha1, size)| (url, ruta, sha1, size))
        .collect();

    let bytes_total: u64 = pendientes.iter().map(|(_, _, _, size)| *size).sum();
    let estado = nuevo_estado_progreso(bytes_total, pendientes.len());
    let semaforo = Arc::new(Semaphore::new(concurrencia));
    let mut tareas = Vec::new();

    for (url, ruta, sha1, _size) in pendientes {
        if *cancelacion.lock().unwrap() {
            break;
        }

        let semaforo_clon = semaforo.clone();
        let app_clon = app.clone();
        let cancelacion_clon = cancelacion.clone();
        let estado_clon = estado.clone();
        let fase = fase.to_string();

        tareas.push(tokio::spawn(async move {
            let _permiso = semaforo_clon.acquire().await.unwrap();
            if *cancelacion_clon.lock().unwrap() {
                return;
            }
            if let Some(padre) = ruta.parent() {
                tokio::fs::create_dir_all(padre).await.ok();
            }
            descargar_archivo(&url, &ruta, &sha1, &app_clon, &fase, &estado_clon).await.ok();
            estado_clon.files_done.fetch_add(1, Ordering::Relaxed);
        }));
    }

    futures::future::join_all(tareas).await;
}

async fn instalar_fabric(
    ruta_juego: &PathBuf,
    version_mc: &str,
    version_loader: &str,
    app: &AppHandle,
) -> Result<String, String> {
    let version_real = if version_loader == "latest" {
        let entradas: Vec<EntradaLoaderFabric> = reqwest::get(format!(
            "https://meta.fabricmc.net/v2/versions/loader/{version_mc}"
        ))
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| format!("Fabric meta parse: {e}"))?;

        entradas
            .into_iter()
            .next()
            .ok_or("No se encontró versión de Fabric")?
            .loader
            .version
    } else {
        version_loader.to_string()
    };

    let perfil: PerfilFabric = reqwest::get(format!(
        "https://meta.fabricmc.net/v2/versions/loader/{version_mc}/{version_real}/profile/json"
    ))
    .await
    .map_err(|e| e.to_string())?
    .json()
    .await
    .map_err(|e| format!("Fabric profile parse: {e}"))?;

    let ruta_librerias = ruta_juego.join("libraries");
    let mut archivos_libreria: Vec<(String, PathBuf, Option<String>, Option<u64>)> = Vec::new();

    for libreria in &perfil.libraries {
        let ruta_relativa = maven_a_ruta(&libreria.name);
        let destino = ruta_librerias.join(&ruta_relativa);
        let url = format!("{}{}", libreria.url.trim_end_matches('/'), format!("/{ruta_relativa}"));
        archivos_libreria.push((url, destino, libreria.sha1.clone(), None));
    }

    emit_progreso(
        app,
        "loader",
        &format!("Descargando Fabric {version_real}..."),
        0,
        0,
        0,
        archivos_libreria.len(),
    );

    let cancelacion = Arc::new(Mutex::new(false));
    ejecutar_descargas_concurrentes(&archivos_libreria, 8, &cancelacion, app, "loader").await;

    app.emit(
        "game-log",
        format!("[Launcher] Fabric {version_real} instalado para MC {version_mc}"),
    )
    .ok();

    Ok(perfil.main_class)
}

async fn instalar_forge(
    ruta_juego: &PathBuf,
    version_mc: &str,
    version_loader: &str,
    ruta_java: &str,
    loader: &str,
    app: &AppHandle,
) -> Result<String, String> {
    let version_forge = if version_loader == "latest" {
        obtener_ultima_forge(version_mc).await?
    } else {
        version_loader.to_string()
    };

    let id_forge = format!("{version_mc}-{version_forge}");
    let directorio_version_forge = ruta_juego.join("versions").join(&id_forge);

    let ruta_json_forge = directorio_version_forge.join(format!("{id_forge}.json"));
    if ruta_json_forge.exists() {
        if let Ok(texto) = tokio::fs::read_to_string(&ruta_json_forge).await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&texto) {
                if let Some(clase_principal) = json["mainClass"].as_str() {
                    app.emit(
                        "game-log",
                        format!("[Launcher] Forge {id_forge} ya instalado"),
                    )
                    .ok();
                    descargar_librerias_forge(ruta_juego, &json, app).await;
                    return Ok(clase_principal.to_string());
                }
            }
        }
    }

    let url_instalador = if loader == "neoforge" {
        format!(
            "https://maven.neoforged.net/releases/net/neoforged/neoforge/{version_forge}/neoforge-{version_forge}-installer.jar"
        )
    } else {
        format!(
            "https://maven.minecraftforge.net/net/minecraftforge/forge/{id_forge}/forge-{id_forge}-installer.jar"
        )
    };

    let ruta_instalador = ruta_juego.join(format!("forge-{id_forge}-installer.jar"));

    emit_progreso(
        app,
        "loader",
        &format!("Descargando Forge {id_forge} installer..."),
        0,
        0,
        0,
        1,
    );

    let tamano_instalador = obtener_content_length(&url_instalador).await.unwrap_or(0);
    let estado_instalador = nuevo_estado_progreso(tamano_instalador, 1);
    descargar_archivo(&url_instalador, &ruta_instalador, &None, app, "loader", &estado_instalador).await?;

    emit_progreso(
        app,
        "loader",
        &format!("Ejecutando Forge installer..."),
        0,
        0,
        0,
        1,
    );

    let cadena_dir_juego = ruta_juego.to_string_lossy().to_string();
    let mut comando_instalador = tokio::process::Command::new(ruta_java);
    comando_instalador.args([
        "-jar",
        &ruta_instalador.to_string_lossy(),
        "--installClient",
        &cadena_dir_juego,
    ]).current_dir(ruta_juego);

    #[cfg(target_os = "windows")]
    {
        comando_instalador.creation_flags(0x08000000);
    }

    let estado = comando_instalador
        .output()
        .await
        .map_err(|e| format!("Error ejecutando Forge installer: {e}"))?;

    if !estado.status.success() {
        let error_estandar = String::from_utf8_lossy(&estado.stderr);
        return Err(format!("Forge installer falló: {error_estandar}"));
    }

    tokio::fs::remove_file(&ruta_instalador).await.ok();

    if ruta_json_forge.exists() {
        let texto = tokio::fs::read_to_string(&ruta_json_forge)
            .await
            .map_err(|e| e.to_string())?;
        let json: serde_json::Value =
            serde_json::from_str(&texto).map_err(|e| e.to_string())?;

        descargar_librerias_forge(ruta_juego, &json, app).await;

        let clase_principal = json["mainClass"]
            .as_str()
            .unwrap_or("cpw.mods.bootstraplauncher.BootstrapLauncher")
            .to_string();

        app.emit(
            "game-log",
            format!("[Launcher] Forge {id_forge} instalado"),
        )
        .ok();

        return Ok(clase_principal);
    }

    Err(format!("Forge instalado pero no se encontró {ruta_json_forge:?}"))
}

async fn obtener_ultima_forge(version_mc: &str) -> Result<String, String> {
    let promociones: PromocionesForge = reqwest::get(
        "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json",
    )
    .await
    .map_err(|e| e.to_string())?
    .json()
    .await
    .map_err(|e| format!("Forge promotions parse: {e}"))?;

    let clave_recomendada = format!("{version_mc}-recommended");
    let clave_ultima = format!("{version_mc}-latest");

    promociones
        .promos
        .get(&clave_recomendada)
        .or_else(|| promociones.promos.get(&clave_ultima))
        .cloned()
        .ok_or(format!("No se encontró Forge para MC {version_mc}"))
}

async fn descargar_librerias_forge(
    ruta_juego: &PathBuf,
    json_forge: &serde_json::Value,
    app: &AppHandle,
) {
    let librerias = match json_forge["libraries"].as_array() {
        Some(l) => l,
        None => return,
    };

    let ruta_librerias = ruta_juego.join("libraries");
    let mut archivos_libreria: Vec<(String, PathBuf, Option<String>, Option<u64>)> = Vec::new();

    for libreria in librerias {
        if let Some(url) = libreria["downloads"]["artifact"]["url"].as_str() {
            if let Some(ruta) = libreria["downloads"]["artifact"]["path"].as_str() {
                let destino = ruta_librerias.join(ruta);
                let sha1 = libreria["downloads"]["artifact"]["sha1"].as_str().map(|s| s.to_string());
                let size = libreria["downloads"]["artifact"]["size"].as_u64();
                archivos_libreria.push((url.to_string(), destino, sha1, size));
            }
        } else if let Some(nombre) = libreria["name"].as_str() {
            let url_base = libreria["url"].as_str().unwrap_or("https://libraries.minecraft.net/");
            let relativa = maven_a_ruta(nombre);
            let url = format!("{}/{relativa}", url_base.trim_end_matches('/'));
            let destino = ruta_librerias.join(&relativa);
            archivos_libreria.push((url, destino, None, None));
        }
    }

    let cancelacion = Arc::new(Mutex::new(false));
    ejecutar_descargas_concurrentes(&archivos_libreria, 8, &cancelacion, app, "loader").await;
}

async fn descargar_archivos_servidor(
    url_instancia: &str,
    ruta_juego: &PathBuf,
    ignorados: &[String],
    max_concurrent: usize,
    cancelacion: &Arc<Mutex<bool>>,
    app: &AppHandle,
) -> Result<usize, String> {
    let respuesta = reqwest::get(url_instancia)
        .await
        .map_err(|e| e.to_string())?;

    let estado = respuesta.status();
    let cuerpo = respuesta.text().await.map_err(|e| e.to_string())?;

    app.emit("game-log", format!("[Launcher] Servidor respondió HTTP {estado} ({} bytes)", cuerpo.len())).ok();

    let archivos: Vec<ArchivoServidor> = serde_json::from_str(&cuerpo)
        .map_err(|e| format!("Parse archivos (HTTP {estado}): {e} — respuesta: {}", &cuerpo[..cuerpo.len().min(200)]))?;

    app.emit("game-log", format!("[Launcher] Total archivos servidor: {}", archivos.len())).ok();

    let base_url = Url::parse(url_instancia).map_err(|e| format!("Base URL parse failed: {e}"))?;

    let candidatos: Vec<(String, PathBuf, Option<String>, u64)> = archivos
        .iter()
        .filter_map(|archivo_servidor| {
            let ruta_relativa = archivo_servidor.path.replace('\\', "/");

            let esta_ignorado = ignorados.iter().any(|ignorado| {
                let ignorado_normalizado = ignorado.trim_start_matches('/').to_lowercase();
                ruta_relativa.to_lowercase().starts_with(&ignorado_normalizado)
            });
            if esta_ignorado {
                return None;
            }

            let url = if let Ok(parsed) = Url::parse(&archivo_servidor.url) {
                parsed.to_string()
            } else {
                base_url.join(&archivo_servidor.url).ok()?.to_string()
            };
            let destino = ruta_juego.join(&ruta_relativa);
            Some((url, destino, Some(archivo_servidor.hash.clone()), archivo_servidor.size))
        })
        .collect();

    let cantidad_ignorados = archivos.len() - candidatos.len();
    if cantidad_ignorados > 0 {
        app.emit("game-log", format!("[Launcher] {cantidad_ignorados} archivos omitidos (carpetas del jugador)")).ok();
    }

    let concurrencia = max_concurrent.max(1).min(16);

    let pendientes: Vec<(String, PathBuf, Option<String>, u64)> = stream::iter(candidatos.into_iter())
        .map(|(url, destino, sha1, size)| async move {
            let jar_corrupto = destino.exists()
                && destino.extension().and_then(|e| e.to_str()) == Some("jar")
                && !es_zip_valido(&destino);
            if jar_corrupto {
                let _ = tokio::fs::remove_file(&destino).await;
            }
            let necesita = jar_corrupto || necesita_descarga(&destino, &sha1).await;
            (necesita, url, destino, sha1, size)
        })
        .buffer_unordered(concurrencia)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .filter(|(necesita, ..)| *necesita)
        .map(|(_, url, destino, sha1, size)| (url, destino, sha1, size))
        .collect();

    let total_archivos = pendientes.len();
    let bytes_total: u64 = pendientes.iter().map(|(_, _, _, size)| *size).sum();
    app.emit("game-log", format!("[Launcher] Archivos a sincronizar: {total_archivos}")).ok();

    emit_progreso(
        app,
        "instance",
        &format!("Sincronizando {total_archivos} archivos..."),
        0,
        bytes_total,
        0,
        total_archivos,
    );

    let estado = nuevo_estado_progreso(bytes_total, total_archivos);
    let semaforo = Arc::new(Semaphore::new(concurrencia));
    let mut tareas = Vec::new();

    for (url, destino, sha1, _size) in pendientes {
        if *cancelacion.lock().unwrap() {
            break;
        }

        let semaforo_clon = semaforo.clone();
        let cancelacion_clon = cancelacion.clone();
        let app_clon = app.clone();
        let estado_clon = estado.clone();

        tareas.push(tokio::spawn(async move {
            let _permiso = semaforo_clon.acquire().await.unwrap();
            if *cancelacion_clon.lock().unwrap() {
                return;
            }

            if let Some(padre) = destino.parent() {
                tokio::fs::create_dir_all(padre).await.ok();
            }
            if let Err(e) = descargar_archivo(&url, &destino, &sha1, &app_clon, "instance", &estado_clon).await {
                app_clon.emit("game-log", format!("[DL ERROR] {}: {e}", destino.display())).ok();
            }

            estado_clon.files_done.fetch_add(1, Ordering::Relaxed);
        }));
    }

    futures::future::join_all(tareas).await;
    Ok(total_archivos)
}

fn maven_a_ruta(nombre: &str) -> String {
    let partes: Vec<&str> = nombre.splitn(4, ':').collect();
    if partes.len() < 3 {
        return nombre.replace(':', "/");
    }
    let grupo = partes[0].replace('.', "/");
    let artefacto = partes[1];
    let version = partes[2];
    let clasificador = if partes.len() == 4 {
        format!("-{}", partes[3])
    } else {
        String::new()
    };
    format!("{grupo}/{artefacto}/{version}/{artefacto}-{version}{clasificador}.jar")
}

async fn necesita_descarga(ruta: &PathBuf, sha1: &Option<String>) -> bool {
    if !ruta.exists() {
        return true;
    }
    if let Some(esperado) = sha1 {
        match tokio::fs::read(ruta).await {
            Ok(datos) => {
                let calculado = hex::encode(Sha1::digest(&datos));
                calculado != *esperado
            }
            Err(_) => true,
        }
    } else {
        false
    }
}

async fn descargar_archivo(
    url: &str,
    ruta: &PathBuf,
    sha1: &Option<String>,
    app: &AppHandle,
    fase: &str,
    estado: &Arc<EstadoProgresoGlobal>,
) -> Result<(), String> {
    let nombre_archivo = ruta
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(url)
        .to_string();

    let respuesta = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !respuesta.status().is_success() {
        return Err(format!("HTTP {} for {url}", respuesta.status()));
    }

    if let Some(padre) = ruta.parent() {
        tokio::fs::create_dir_all(padre).await.map_err(|e| e.to_string())?;
    }

    let tmp_path = ruta.with_extension(
        ruta.extension()
            .map(|ext| format!("{}.part", ext.to_string_lossy()))
            .unwrap_or_else(|| "part".to_string()),
    );

    let mut archivo = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| e.to_string())?;

    let mut hasher = Sha1::new();
    let mut flujo = respuesta.bytes_stream();

    while let Some(fragmento) = flujo.next().await {
        let fragmento = fragmento.map_err(|e| e.to_string())?;
        archivo.write_all(&fragmento).await.map_err(|e| e.to_string())?;
        hasher.update(&fragmento);
        let descargado_global = estado.bytes_done.fetch_add(fragmento.len() as u64, Ordering::Relaxed)
            + fragmento.len() as u64;

        let transcurrido = estado.inicio.elapsed().as_secs_f64();
        let velocidad = if transcurrido > 0.0 {
            descargado_global as f64 / transcurrido
        } else {
            0.0
        };
        let restante = if velocidad > 0.0 && estado.bytes_total > descargado_global {
            (estado.bytes_total - descargado_global) as f64 / velocidad
        } else {
            0.0
        };

        app.emit(
            "download-progress",
            ProgresoDescarga {
                phase: fase.to_string(),
                file: nombre_archivo.clone(),
                downloaded: descargado_global,
                total: estado.bytes_total,
                speed_bps: velocidad,
                eta_seconds: restante,
                files_done: estado.files_done.load(Ordering::Relaxed),
                files_total: estado.files_total,
            },
        )
        .ok();
    }

    if let Some(esperado) = sha1 {
        let calculado = hex::encode(hasher.finalize());
        if calculado != *esperado {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(format!("SHA1 mismatch for {nombre_archivo}: expected {esperado}, got {calculado}"));
        }
    }

    tokio::fs::rename(&tmp_path, ruta)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn es_zip_valido(ruta: &std::path::Path) -> bool {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut archivo) = std::fs::File::open(ruta) else { return false };
    let mut firma = [0u8; 4];
    if archivo.read_exact(&mut firma).is_err() { return false }
    if &firma != b"PK\x03\x04" { return false }
    let Ok(longitud) = archivo.seek(SeekFrom::End(0)) else { return false };
    if longitud < 22 { return false }
    let inicio_escaneo = longitud.saturating_sub(65558);
    let _ = archivo.seek(SeekFrom::Start(inicio_escaneo));
    let mut cola = Vec::new();
    let _ = archivo.read_to_end(&mut cola);
    cola.windows(4).any(|w| w == b"PK\x05\x06")
}

fn emit_progreso(
    app: &AppHandle,
    fase: &str,
    archivo: &str,
    descargado: u64,
    total: u64,
    archivos_completados: usize,
    archivos_totales: usize,
) {
    app.emit(
        "download-progress",
        ProgresoDescarga {
            phase: fase.to_string(),
            file: archivo.to_string(),
            downloaded: descargado,
            total,
            speed_bps: 0.0,
            eta_seconds: 0.0,
            files_done: archivos_completados,
            files_total: archivos_totales,
        },
    )
    .ok();
}

async fn obtener_content_length(url: &str) -> Option<u64> {
    reqwest::Client::new()
        .head(url)
        .send()
        .await
        .ok()?
        .content_length()
}

fn obtener_nombre_os() -> &'static str {
    #[cfg(target_os = "windows")]
    return "windows";
    #[cfg(target_os = "macos")]
    return "osx";
    #[cfg(target_os = "linux")]
    return "linux";
}

fn verificar_reglas_aplican(reglas: &Option<Vec<Regla>>, nombre_os: &str) -> bool {
    let Some(reglas) = reglas else { return true };
    if reglas.is_empty() {
        return true;
    }
    let mut resultado = false;
    for regla in reglas {
        let coincide_os = match &regla.os {
            Some(regla_os) => regla_os.name.as_deref() == Some(nombre_os),
            None => true,
        };
        if regla.action == "allow" && coincide_os {
            resultado = true;
        } else if regla.action == "disallow" && coincide_os {
            resultado = false;
        }
    }
    resultado
}
