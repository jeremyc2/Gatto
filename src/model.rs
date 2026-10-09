use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, bail};
use gpui_kit::{Image, ImageFormat};

static NEXT_STAGED_IMAGE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repository {
    pub id: u64,
    pub name: String,
}

/// Validates a GitHub organization login and returns it trimmed.
pub fn parse_organization(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        bail!("Enter a GitHub organization name.");
    }
    Ok(value.to_owned())
}

#[derive(Clone)]
pub struct StagedImage {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub upload_state: AttachmentUploadState,
    pub mime_type: String,
    pub bytes: Arc<Vec<u8>>,
    pub preview: Arc<Image>,
    pub pixel_size: (u32, u32),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AttachmentUploadState {
    #[default]
    Preview,
    Uploading,
    Uploaded {
        url: String,
    },
    Failed {
        message: String,
    },
}

impl AttachmentUploadState {
    pub fn is_uploaded(&self) -> bool {
        matches!(self, Self::Uploaded { .. })
    }

    pub fn uploaded_url(&self) -> Option<&str> {
        match self {
            Self::Uploaded { url } => Some(url),
            _ => None,
        }
    }

    pub fn failure_message(&self) -> Option<&str> {
        match self {
            Self::Failed { message } => Some(message),
            _ => None,
        }
    }
}

impl StagedImage {
    pub fn from_path(path: PathBuf) -> Result<Self> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .context("The selected image has an invalid file name")?
            .to_owned();
        let bytes =
            std::fs::read(&path).with_context(|| format!("Could not read {}", path.display()))?;
        Self::from_bytes(name, bytes, Some(&path))
    }

    pub fn from_clipboard(image: &Image) -> Result<Self> {
        let format = supported_gpui_format(image.format)
            .context("Unsupported format. Please paste or drop an image file (PNG/JPEG).")?;
        let pixel_size = validate_image_bytes(&image.bytes, format)?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let name = format!("clipboard-{timestamp}.{}", format.extension());
        let bytes = Arc::new(image.bytes.clone());

        Ok(Self {
            id: NEXT_STAGED_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
            description: name.clone(),
            name,
            upload_state: AttachmentUploadState::Preview,
            mime_type: format.mime_type().to_owned(),
            preview: Arc::new(Image::from_bytes(format, bytes.as_ref().clone())),
            bytes,
            pixel_size,
        })
    }

    fn from_bytes(name: String, bytes: Vec<u8>, path: Option<&Path>) -> Result<Self> {
        let guessed = ::image::guess_format(&bytes)
            .context("Unsupported format. Please paste or drop an image file (PNG/JPEG).")?;
        let format = match guessed {
            ::image::ImageFormat::Png => ImageFormat::Png,
            ::image::ImageFormat::Jpeg => ImageFormat::Jpeg,
            ::image::ImageFormat::Gif => ImageFormat::Gif,
            ::image::ImageFormat::WebP => ImageFormat::Webp,
            _ => bail!("Unsupported format. Please paste or drop an image file (PNG/JPEG)."),
        };

        if let Some(path) = path {
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp") {
                bail!("Unsupported format. Please paste or drop an image file (PNG/JPEG).");
            }
        }

        let pixel_size = validate_image_bytes(&bytes, format)?;
        let bytes = Arc::new(bytes);
        Ok(Self {
            id: NEXT_STAGED_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
            description: name.clone(),
            name,
            upload_state: AttachmentUploadState::Preview,
            mime_type: format.mime_type().to_owned(),
            preview: Arc::new(Image::from_bytes(format, bytes.as_ref().clone())),
            bytes,
            pixel_size,
        })
    }

    pub fn formatted_size(&self) -> String {
        let bytes = self.bytes.len() as f64;
        if bytes >= 1_048_576.0 {
            format!("{:.1} MB", bytes / 1_048_576.0)
        } else {
            format!("{:.0} KB", (bytes / 1024.0).max(0.1))
        }
    }
}

fn supported_gpui_format(format: ImageFormat) -> Option<ImageFormat> {
    match format {
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::Webp => Some(format),
        _ => None,
    }
}

fn validate_image_bytes(bytes: &[u8], format: ImageFormat) -> Result<(u32, u32)> {
    if bytes.is_empty() {
        bail!("The image is empty.");
    }

    let image_format = match format {
        ImageFormat::Png => ::image::ImageFormat::Png,
        ImageFormat::Jpeg => ::image::ImageFormat::Jpeg,
        ImageFormat::Gif => ::image::ImageFormat::Gif,
        ImageFormat::Webp => ::image::ImageFormat::WebP,
        _ => bail!("Unsupported format. Please paste or drop an image file (PNG/JPEG)."),
    };
    let image = ::image::load_from_memory_with_format(bytes, image_format)
        .context("The image data is corrupt or unreadable")?;
    Ok((image.width(), image.height()))
}

#[cfg(test)]
mod attachment_upload_state_tests {
    use super::AttachmentUploadState;

    #[test]
    fn only_uploaded_state_exposes_a_url() {
        let uploaded = AttachmentUploadState::Uploaded {
            url: "https://example.com/image".into(),
        };
        let failed = AttachmentUploadState::Failed {
            message: "network error".into(),
        };

        assert!(uploaded.is_uploaded());
        assert_eq!(uploaded.uploaded_url(), Some("https://example.com/image"));
        assert_eq!(failed.failure_message(), Some("network error"));
        assert!(!failed.is_uploaded());
    }
}
