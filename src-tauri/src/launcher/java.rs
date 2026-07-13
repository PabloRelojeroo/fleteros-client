use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct InstalacionJava {
    pub path: String,
    pub version: String,
}

#[tauri::command]
pub async fn get_java_paths() -> Result<Vec<InstalacionJava>, String> {
    Ok(detectar_instalaciones_java())
}

#[tauri::command]
pub async fn get_best_java_for_version(mc_version: String) -> Result<String, String> {
    let requerido = version_java_requerida(&mc_version);
    let instalaciones = detectar_instalaciones_java();

    let mejor = instalaciones
        .iter()
        .filter_map(|j| {
            let mayor = version_mayor_java(&j.version)?;
            if mayor >= requerido { Some((mayor, j.path.clone())) } else { None }
        })
        .min_by_key(|(mayor, _)| *mayor);

    Ok(mejor.map(|(_, p)| p).unwrap_or_else(|| "java".to_string()))
}

fn detectar_instalaciones_java() -> Vec<InstalacionJava> {
    let mut instalaciones: Vec<InstalacionJava> = Vec::new();

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let claves_busqueda = [
            r"SOFTWARE\JavaSoft\Java Runtime Environment",
            r"SOFTWARE\JavaSoft\JDK",
            r"SOFTWARE\WOW6432Node\JavaSoft\Java Runtime Environment",
            r"SOFTWARE\WOW6432Node\JavaSoft\JDK",
        ];

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        for ruta_clave in &claves_busqueda {
            if let Ok(clave) = hklm.open_subkey(ruta_clave) {
                for nombre_subclave in clave.enum_keys().flatten() {
                    if let Ok(subclave) = clave.open_subkey(&nombre_subclave) {
                        if let Ok(directorio_java) = subclave.get_value::<String, _>("JavaHome") {
                            let ejecutable = PathBuf::from(&directorio_java).join("bin").join("java.exe");
                            if ejecutable.exists() {
                                if let Some(instalacion) = sondear_java(&ejecutable.to_string_lossy()) {
                                    if !instalaciones.iter().any(|i| i.path == instalacion.path) {
                                        instalaciones.push(instalacion);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let rutas_comunes = [
            r"C:\Program Files\Java",
            r"C:\Program Files\Eclipse Adoptium",
            r"C:\Program Files\Microsoft",
            r"C:\Program Files\Eclipse Foundation",
            r"C:\Program Files\Zulu",
        ];
        for base in &rutas_comunes {
            if let Ok(entradas) = std::fs::read_dir(base) {
                for entrada in entradas.flatten() {
                    let ejecutable = entrada.path().join("bin").join("java.exe");
                    if ejecutable.exists() {
                        if let Some(instalacion) = sondear_java(&ejecutable.to_string_lossy()) {
                            if !instalaciones.iter().any(|i| i.path == instalacion.path) {
                                instalaciones.push(instalacion);
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        for ruta_java in ["/usr/bin/java", "/usr/local/bin/java"] {
            if std::path::Path::new(ruta_java).exists() {
                if let Some(instalacion) = sondear_java(ruta_java) {
                    instalaciones.push(instalacion);
                }
            }
        }
        if let Ok(directorio_java) = std::env::var("JAVA_HOME") {
            let ejecutable = PathBuf::from(&directorio_java).join("bin").join("java");
            if ejecutable.exists() {
                if let Some(instalacion) = sondear_java(&ejecutable.to_string_lossy()) {
                    if !instalaciones.iter().any(|i| i.path == instalacion.path) {
                        instalaciones.push(instalacion);
                    }
                }
            }
        }
    }

    instalaciones
}

fn sondear_java(path: &str) -> Option<InstalacionJava> {
    let salida = Command::new(path).arg("-version").output().ok()?;
    let texto_version = String::from_utf8_lossy(&salida.stderr).to_string();
    let version = parsear_version_java(&texto_version).unwrap_or_else(|| "unknown".to_string());
    Some(InstalacionJava { path: path.to_string(), version })
}

fn parsear_version_java(salida: &str) -> Option<String> {
    let linea = salida.lines().next()?;
    let inicio = linea.find('"')? + 1;
    let fin = linea.rfind('"')?;
    Some(linea[inicio..fin].to_string())
}

fn version_java_requerida(mc_version: &str) -> u32 {
    let partes: Vec<u32> = mc_version
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    let menor = partes.get(1).copied().unwrap_or(0);
    let parche = partes.get(2).copied().unwrap_or(0);

    if menor >= 21 || (menor == 20 && parche >= 5) {
        21
    } else if menor >= 17 {
        17
    } else {
        8
    }
}

fn version_mayor_java(version: &str) -> Option<u32> {
    let primero = version.split('.').next()?;
    if primero == "1" {
        version.split('.').nth(1)?.parse().ok()
    } else {
        primero.parse().ok()
    }
}
