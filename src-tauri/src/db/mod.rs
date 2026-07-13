use crate::auth::Account;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct EstadoBd(pub Mutex<Connection>);

pub fn init(app: &AppHandle) -> anyhow::Result<()> {
    let ruta = ruta_bd(app);
    std::fs::create_dir_all(ruta.parent().unwrap())?;
    let conexion = Connection::open(&ruta)?;

    conexion.execute_batch(
        "CREATE TABLE IF NOT EXISTS accounts (
            id TEXT PRIMARY KEY,
            data TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS access_codes (
            instance_id TEXT PRIMARY KEY,
            code TEXT NOT NULL
        );",
    )?;

    app.manage(EstadoBd(Mutex::new(conexion)));
    Ok(())
}

fn ruta_bd(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap().join("survi.db")
}

#[tauri::command]
pub async fn get_accounts(app: AppHandle) -> Result<Vec<Account>, String> {
    let estado = app.state::<EstadoBd>();
    let conexion = estado.0.lock().map_err(|e| e.to_string())?;
    let mut sentencia = conexion
        .prepare("SELECT data FROM accounts")
        .map_err(|e| e.to_string())?;

    let cuentas = sentencia
        .query_map([], |fila| fila.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .filter_map(|s| serde_json::from_str(&s).ok())
        .collect();

    Ok(cuentas)
}

pub async fn save_account(app: &AppHandle, account: &Account) -> anyhow::Result<()> {
    let estado = app.state::<EstadoBd>();
    let conexion = estado.0.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
    let datos = serde_json::to_string(account)?;
    conexion.execute(
        "INSERT OR REPLACE INTO accounts (id, data) VALUES (?1, ?2)",
        params![account.id, datos],
    )?;
    Ok(())
}

pub async fn delete_account(app: &AppHandle, account_id: &str) -> anyhow::Result<()> {
    let estado = app.state::<EstadoBd>();
    let conexion = estado.0.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
    conexion.execute("DELETE FROM accounts WHERE id = ?1", params![account_id])?;
    Ok(())
}

#[tauri::command]
pub async fn save_account_cmd(app: AppHandle, account: Account) -> Result<(), String> {
    save_account(&app, &account).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_account_cmd(app: AppHandle, account_id: String) -> Result<(), String> {
    delete_account(&app, &account_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_config(app: AppHandle, key: String) -> Result<Option<String>, String> {
    let estado = app.state::<EstadoBd>();
    let conexion = estado.0.lock().map_err(|e| e.to_string())?;
    let resultado = conexion.query_row(
        "SELECT value FROM config WHERE key = ?1",
        params![key],
        |fila| fila.get(0),
    );
    match resultado {
        Ok(valor) => Ok(Some(valor)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn set_config(app: AppHandle, key: String, value: String) -> Result<(), String> {
    let estado = app.state::<EstadoBd>();
    let conexion = estado.0.lock().map_err(|e| e.to_string())?;
    conexion.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
