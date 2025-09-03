// src/services.rs
use crate::error::AppError;
use rfd::AsyncFileDialog;
use std::path::PathBuf;
// ... weitere `use` Anweisungen bleiben gleich ...

// Funktion zum Öffnen des Dateidialogs
pub async fn pick_file() -> Option<PathBuf> {
    let file = AsyncFileDialog::new().pick_file().await;
    file.map(|f| f.path().to_path_buf())
}

// Funktion zum Speichern des Dateidialogs
pub async fn save_file() -> Option<PathBuf> {
    let file = AsyncFileDialog::new().save_file().await;
    file.map(|f| f.path().to_path_buf())
}


// Die Kernlogik wird `async`
pub async fn generate_qr_from_file(
    filepath: PathBuf,
    password: String,
) -> Result<(Vec<u8>, String), AppError> {
    // Diese Funktion kann rechenintensiv sein, daher lagern wir sie auf
    // einen blockierenden Thread aus, um den async-Runtime nicht zu belasten.
    tokio::task::spawn_blocking(move || {
        let raw_data = std::fs::read(filepath)?;
        let encoded_string = super::qr_data::process_data_for_qr(&raw_data, &password)?;
        let image_bytes = super::qr_data::generate_qr_image(&encoded_string)?;
        Ok((image_bytes, encoded_string))
    })
        .await
        .map_err(|e| AppError::Io(e.to_string()))?
}
