use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[tauri::command]
pub async fn verify_access_code(
    instance_id: String,
    code: String,
    secret: String,
) -> Result<bool, String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| e.to_string())?;

    mac.update(instance_id.as_bytes());
    let esperado = hex::encode(mac.finalize().into_bytes());

    Ok(esperado == code.to_lowercase())
}
