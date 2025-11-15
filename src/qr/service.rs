// src/qr/service.rs
use qrcode::{QrCode, EcLevel};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum QrServiceError {
    #[error("QR code generation failed: {0}")]
    GenerationFailed(String),
    #[error("Image read error: {0}")]
    ImageReadError(String),
    #[error("No QR code found in image")]
    QrCodeNotFound,
}

pub fn generate_qr_image(data: &str) -> Result<Vec<u8>, QrServiceError> {
    let code = QrCode::with_error_correction_level(data, EcLevel::L)
        .map_err(|e| QrServiceError::GenerationFailed(e.to_string()))?;

    // Render as image buffer
    let image_buffer = code
        .render::<image::Luma<u8>>()
        .max_dimensions(512, 512)
        .build();

    // Convert to PNG bytes
    let mut buffer = Vec::new();
    let img = image::DynamicImage::ImageLuma8(image_buffer);
    img.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png)
        .map_err(|e| QrServiceError::GenerationFailed(e.to_string()))?;

    Ok(buffer)
}

pub fn read_qr_from_image(filepath: &str) -> Result<String, QrServiceError> {
    let img = image::open(filepath)
        .map_err(|e| QrServiceError::ImageReadError(e.to_string()))?;

    let img = img.to_luma8();

    let mut decoder = rqrr::PreparedImage::prepare(img);
    let grids = decoder.detect_grids();

    if grids.is_empty() {
        return Err(QrServiceError::QrCodeNotFound);
    }

    let (_, content) = grids[0]
        .decode()
        .map_err(|_| QrServiceError::QrCodeNotFound)?;

    Ok(content)
}
