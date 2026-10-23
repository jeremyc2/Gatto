use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, bail};
use gpui_kit::{Image, ImageFormat};

use crate::video::Video;

pub const MAX_ATTACHMENT_BYTES: u64 = 10_000_000;
pub const UNSUPPORTED_MEDIA_MESSAGE: &str = "Unsupported file type.";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VideoUploadLimit {
    #[default]
    Standard,
    Paid,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageCopyFormat {
    #[default]
    Html,
    Markdown,
}

impl VideoUploadLimit {
    pub fn bytes(self) -> u64 {
        match self {
            Self::Standard => MAX_ATTACHMENT_BYTES,
            Self::Paid => 100_000_000,
        }
    }

    pub fn megabytes(self) -> u64 {
        self.bytes() / 1_000_000
    }
}

static NEXT_STAGED_IMAGE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repository {
    pub id: u64,
    pub name: String,
}

/// Validates a GitHub account or organization login and returns it trimmed.
pub fn parse_owner(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        bail!("Enter a GitHub account or organization login.");
    }
    Ok(value.to_owned())
}

#[derive(Debug, thiserror::Error)]
#[error(
    "This video exceeds {limit_mb} MB. Open it in Gatto to choose Compress, or trim it before using Quick Copy."
)]
pub struct OversizedVideo {
    pub path: PathBuf,
    pub bytes: u64,
    pub limit_mb: u64,
}

#[derive(Clone)]
pub struct StagedAttachment {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub upload_state: AttachmentUploadState,
    pub mime_type: String,
    pub media: Media,
}

#[derive(Clone)]
pub enum Media {
    Image(ImageData),
    Video(Arc<Video>),
}

#[derive(Clone)]
pub struct ImageData {
    pub marker_descriptions: Vec<(u32, String)>,
    pub bytes: Arc<[u8]>,
    pub preview: Arc<Image>,
    pub pixel_size: (u32, u32),
    pub annotation_session: crate::annotation::Session,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AttachmentUploadState {
    #[default]
    Preview,
    Queued,
    Uploading {
        sent: u64,
        total: u64,
    },
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

impl StagedAttachment {
    pub fn from_path(path: PathBuf, video_limit: VideoUploadLimit) -> Result<Self> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .context("The selected file has an invalid name")?
            .to_owned();
        let path_metadata = std::fs::metadata(&path)
            .with_context(|| format!("Could not inspect {}", path.display()))?;
        if !path_metadata.is_file() {
            bail!("Choose a regular image or video file, rather than a folder or device.");
        }
        let mut file = std::fs::File::open(&path)
            .with_context(|| format!("Could not read {}", path.display()))?;
        let metadata = file
            .metadata()
            .context("Could not inspect the selected file")?;
        if !metadata.is_file() {
            bail!("Choose a regular image or video file, rather than a folder or device.");
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if let Some(mime_type) = Video::mime_type(&extension) {
            if metadata.len() > video_limit.bytes() {
                return Err(OversizedVideo {
                    path,
                    bytes: metadata.len(),
                    limit_mb: video_limit.megabytes(),
                }
                .into());
            }
            validate_size(metadata.len(), video_limit.bytes())?;
            let video = Video::stage(&mut file, &extension, video_limit)?;
            return Ok(Self {
                id: NEXT_STAGED_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
                description: name.clone(),
                name,
                upload_state: AttachmentUploadState::Preview,
                mime_type: mime_type.into(),
                media: Media::Video(Arc::new(video)),
            });
        }
        validate_size(metadata.len(), MAX_ATTACHMENT_BYTES)?;
        let mut bytes = Vec::new();
        file.take(MAX_ATTACHMENT_BYTES + 1)
            .read_to_end(&mut bytes)
            .context("Could not read the selected image")?;
        validate_size(bytes.len() as u64, MAX_ATTACHMENT_BYTES)?;
        Self::from_bytes(name, bytes, Some(&path))
    }

    pub fn image(&self) -> Option<&ImageData> {
        match &self.media {
            Media::Image(image) => Some(image),
            Media::Video(_) => None,
        }
    }

    pub fn len(&self) -> u64 {
        match &self.media {
            Media::Image(image) => image.bytes.len() as u64,
            Media::Video(video) => video.len(),
        }
    }

    pub fn reader(&self) -> Result<Box<dyn Read + Send>> {
        match &self.media {
            Media::Image(image) => Ok(Box::new(Cursor::new(image.bytes.clone()))),
            Media::Video(video) => Ok(Box::new(video.reader()?)),
        }
    }

    pub fn validate_upload_size(&self, video_limit: VideoUploadLimit) -> Result<()> {
        let limit = match self.media {
            Media::Image(_) => MAX_ATTACHMENT_BYTES,
            Media::Video(_) => video_limit.bytes(),
        };
        validate_size(self.len(), limit)
    }

    pub fn snippet(&self, url: &str, format: ImageCopyFormat) -> String {
        match &self.media {
            Media::Image(image) => {
                let mut snippet = match format {
                    ImageCopyFormat::Html => html_image(&self.description, url, image.pixel_size),
                    ImageCopyFormat::Markdown => markdown_image(&self.description, url),
                };
                for (index, (number, description)) in image.marker_descriptions.iter().enumerate() {
                    snippet.push_str(if index == 0 { "\n\n" } else { "\n" });
                    let prefix = format!("{number}. ");
                    let indent = " ".repeat(prefix.len());
                    snippet.push_str(&prefix);
                    snippet.push_str(
                        &description
                            .replace("\r\n", "\n")
                            .replace('\n', &format!("\n{indent}")),
                    );
                }
                snippet
            }
            Media::Video(_) => url.to_owned(),
        }
    }

    pub fn from_clipboard(image: &Image) -> Result<Self> {
        validate_size(image.bytes.len() as u64, MAX_ATTACHMENT_BYTES)?;
        let format = supported_gpui_format(image.format).context(UNSUPPORTED_MEDIA_MESSAGE)?;
        let pixel_size = validate_image_bytes(&image.bytes, format)?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let name = format!("clipboard-{timestamp}.{}", format.extension());
        let bytes: Arc<[u8]> = image.bytes.clone().into();

        Ok(Self {
            id: NEXT_STAGED_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
            description: name.clone(),
            name,
            upload_state: AttachmentUploadState::Preview,
            mime_type: format.mime_type().to_owned(),
            media: Media::Image(ImageData {
                marker_descriptions: Vec::new(),
                annotation_session: Default::default(),
                preview: Arc::new(Image::from_bytes(format, bytes.to_vec())),
                bytes,
                pixel_size,
            }),
        })
    }

    pub(crate) fn from_bytes(name: String, bytes: Vec<u8>, path: Option<&Path>) -> Result<Self> {
        let guessed = ::image::guess_format(&bytes).context(UNSUPPORTED_MEDIA_MESSAGE)?;
        let format = match guessed {
            ::image::ImageFormat::Png => ImageFormat::Png,
            ::image::ImageFormat::Jpeg => ImageFormat::Jpeg,
            ::image::ImageFormat::Gif => ImageFormat::Gif,
            ::image::ImageFormat::WebP => ImageFormat::Webp,
            _ => bail!(UNSUPPORTED_MEDIA_MESSAGE),
        };

        if let Some(path) = path {
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp") {
                bail!(UNSUPPORTED_MEDIA_MESSAGE);
            }
        }

        let pixel_size = validate_image_bytes(&bytes, format)?;
        Ok(Self::from_validated_image(name, bytes, format, pixel_size))
    }

    /// Only for PNG bytes and dimensions returned by our own renderer.
    pub(crate) fn from_rendered_png(
        name: String,
        bytes: Vec<u8>,
        pixel_size: (u32, u32),
    ) -> Result<Self> {
        validate_size(bytes.len() as u64, MAX_ATTACHMENT_BYTES)?;
        Ok(Self::from_validated_image(
            name,
            bytes,
            ImageFormat::Png,
            pixel_size,
        ))
    }

    fn from_validated_image(
        name: String,
        bytes: Vec<u8>,
        format: ImageFormat,
        pixel_size: (u32, u32),
    ) -> Self {
        let bytes: Arc<[u8]> = bytes.into();
        Self {
            id: NEXT_STAGED_IMAGE_ID.fetch_add(1, Ordering::Relaxed),
            description: name.clone(),
            name,
            upload_state: AttachmentUploadState::Preview,
            mime_type: format.mime_type().to_owned(),
            media: Media::Image(ImageData {
                marker_descriptions: Vec::new(),
                annotation_session: Default::default(),
                preview: Arc::new(Image::from_bytes(format, bytes.to_vec())),
                bytes,
                pixel_size,
            }),
        }
    }

    pub fn formatted_size(&self) -> String {
        let bytes = self.len() as f64;
        if bytes >= 1_000_000.0 {
            format!("{:.1} MB", bytes / 1_000_000.0)
        } else {
            format!("{:.0} KB", (bytes / 1000.0).max(0.1))
        }
    }
}

pub fn validate_size(len: u64, limit: u64) -> Result<()> {
    if len == 0 {
        bail!("The selected file is empty.");
    }
    if len > limit {
        bail!(
            "The selected file exceeds the {} MB limit. Remove it and add it again to choose compression, or trim it and try again.",
            limit / 1_000_000
        );
    }
    Ok(())
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
        _ => bail!(UNSUPPORTED_MEDIA_MESSAGE),
    };
    let image = ::image::load_from_memory_with_format(bytes, image_format)
        .context("The image data is corrupt or unreadable")?;
    Ok((image.width(), image.height()))
}

/// HTML supported in GitHub Markdown, sized to the validated image pixels.
pub fn html_image(description: &str, url: &str, pixel_size: (u32, u32)) -> String {
    let alt_text = escape_html_attribute(&description.replace(['\r', '\n'], " "));
    let src = escape_html_attribute(url);
    let (width, height) = pixel_size;
    format!(r#"<img width="{width}" height="{height}" alt="{alt_text}" src="{src}" />"#)
}

fn markdown_image(description: &str, url: &str) -> String {
    let alt_text = description
        .replace(['\r', '\n'], " ")
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('`', "\\`")
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    // Keep the destination a single Markdown link even if its URL contains
    // whitespace or parentheses. GitHub attachment URLs normally contain neither.
    let destination = url
        .replace('\\', "%5C")
        .replace('(', "%28")
        .replace(')', "%29")
        .replace(' ', "%20")
        .replace('<', "%3C")
        .replace('>', "%3E")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace('\t', "%09");
    format!("![{alt_text}]({destination})")
}

fn escape_html_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod image_snippet_tests {
    use super::{html_image, markdown_image};

    #[test]
    fn seven_markers_copy_all_numbers_with_only_fifth_or_no_description() {
        use super::{ImageCopyFormat, Media, StagedAttachment};
        use crate::annotation::{Document, Mark, Pos, Style, Tool};
        for fifth in ["Choose a repository", ""] {
            let mut doc = Document::from_image(image::RgbaImage::new(100, 100));
            for i in 1..=7 {
                let mut mark = Mark::new(Tool::Marker, Pos::new(50., 50.), Style::default());
                doc.number_marker(&mut mark);
                if i == 5 {
                    mark.description = fifth.into();
                }
                doc.marks.push(mark);
            }
            let mut item = StagedAttachment::from_bytes(
                "test.png".into(),
                include_bytes!("../packaging/menu-bar-icon.png").to_vec(),
                None,
            )
            .unwrap();
            if let Media::Image(image) = &mut item.media {
                image.marker_descriptions = doc.marker_descriptions();
            }
            assert_eq!(item.image().unwrap().marker_descriptions.len(), 7);
            let expected = (1..=7)
                .map(|i| format!("{i}. {}", if i == 5 { fifth } else { "" }))
                .collect::<Vec<_>>()
                .join("\n");
            for format in [ImageCopyFormat::Html, ImageCopyFormat::Markdown] {
                let snippet = item.snippet("https://example.com/image", format);
                assert_eq!(snippet.split_once("\n\n").unwrap().1, expected);
            }
        }
    }

    #[test]
    fn descriptions_copy_below_each_image_with_multiline_continuations() {
        use super::{ImageCopyFormat, Media, StagedAttachment};
        let mut item = StagedAttachment::from_bytes(
            "test.png".into(),
            include_bytes!("../packaging/menu-bar-icon.png").to_vec(),
            None,
        )
        .unwrap();
        if let Media::Image(image) = &mut item.media {
            image.marker_descriptions = vec![
                (2, "First line\nSecond line\n\nNew paragraph".into()),
                (10, "Last".into()),
            ];
        }
        for format in [ImageCopyFormat::Html, ImageCopyFormat::Markdown] {
            let snippet = item.snippet("https://example.com/image", format);
            assert!(
                snippet.ends_with(
                    "\n\n2. First line\n   Second line\n   \n   New paragraph\n10. Last"
                )
            );
            assert!(!snippet.contains("Marker"));
        }
    }

    #[test]
    fn preserves_dimensions_and_escapes_html_attributes() {
        assert_eq!(
            html_image(
                "A [cat] \\ \"quoted\" & <sleeping> 'cat'\nnext line",
                "https://github.com/user-attachments/assets/example?a=1&b=\"2\"",
                (640, 480),
            ),
            r#"<img width="640" height="480" alt="A [cat] \ &quot;quoted&quot; &amp; &lt;sleeping&gt; &#39;cat&#39; next line" src="https://github.com/user-attachments/assets/example?a=1&amp;b=&quot;2&quot;" />"#,
        );
    }

    #[test]
    fn markdown_has_alt_text_without_dimensions_and_escapes_markup() {
        assert_eq!(
            markdown_image(
                "Gatto rocket cat",
                "https://github.com/user-attachments/assets/cat"
            ),
            "![Gatto rocket cat](https://github.com/user-attachments/assets/cat)"
        );
        assert_eq!(
            markdown_image(
                "[cat] \\ *code* _name_ `x` & <tag>\nnext",
                "https://example.com/a (b).png"
            ),
            r"![\[cat\] \\ \*code\* \_name\_ \`x\` &amp; &lt;tag&gt; next](https://example.com/a%20%28b%29.png)"
        );
    }
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

#[cfg(test)]
mod media_tests {
    use super::*;
    use std::io::{Seek as _, Write as _};

    pub(crate) const VIDEO: &[u8] = include_bytes!("../tests/fixtures/sample.mp4");

    #[test]
    fn unsupported_file_types_have_a_concise_error() {
        let mut file = tempfile::Builder::new().suffix(".txt").tempfile().unwrap();
        file.write_all(b"not a supported media file").unwrap();

        let error =
            match StagedAttachment::from_path(file.path().into(), VideoUploadLimit::Standard) {
                Ok(_) => panic!("unsupported file type should not be staged"),
                Err(error) => error,
            };

        assert_eq!(error.to_string(), UNSUPPORTED_MEDIA_MESSAGE);
    }

    #[test]
    fn paid_video_limit_preserves_image_limit_and_rechecks_staged_files() {
        let mut file = tempfile::Builder::new().suffix(".mp4").tempfile().unwrap();
        file.write_all(VIDEO).unwrap();
        let padding = MAX_ATTACHMENT_BYTES + 8;
        file.write_all(&(padding as u32).to_be_bytes()).unwrap();
        file.write_all(b"free").unwrap();
        file.as_file()
            .set_len(VIDEO.len() as u64 + padding)
            .unwrap();
        let error = StagedAttachment::from_path(file.path().into(), VideoUploadLimit::Standard)
            .err()
            .unwrap();
        assert_eq!(error.downcast_ref::<OversizedVideo>().unwrap().limit_mb, 10);
        let item = StagedAttachment::from_path(file.path().into(), VideoUploadLimit::Paid).unwrap();
        assert!(item.len() > MAX_ATTACHMENT_BYTES);
        assert!(item.validate_upload_size(VideoUploadLimit::Paid).is_ok());
        assert!(
            item.validate_upload_size(VideoUploadLimit::Standard)
                .is_err()
        );
        let paid_max = VideoUploadLimit::Paid.bytes();
        assert!(validate_size(paid_max, paid_max).is_ok());
        file.as_file().set_len(paid_max + 1).unwrap();
        let error = StagedAttachment::from_path(file.path().into(), VideoUploadLimit::Paid)
            .err()
            .unwrap();
        assert_eq!(
            error.downcast_ref::<OversizedVideo>().unwrap().limit_mb,
            100
        );

        let image = tempfile::Builder::new().suffix(".png").tempfile().unwrap();
        image.as_file().set_len(MAX_ATTACHMENT_BYTES + 1).unwrap();
        let error = StagedAttachment::from_path(image.path().into(), VideoUploadLimit::Paid)
            .err()
            .unwrap();
        assert!(error.to_string().contains("10 MB"));
    }

    #[test]
    fn video_snapshot_survives_source_changes_and_has_repeatable_readers() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recording.MP4");
        std::fs::write(&path, VIDEO).unwrap();
        let item = StagedAttachment::from_path(path.clone(), Default::default()).unwrap();
        std::fs::write(&path, b"source changed after staging").unwrap();
        std::fs::remove_file(path).unwrap();
        assert!(item.image().is_none());
        assert_eq!(item.mime_type, "video/mp4");
        assert_eq!(item.len(), VIDEO.len() as u64);
        for _ in 0..2 {
            let mut bytes = Vec::new();
            item.reader().unwrap().read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes, VIDEO);
        }
        assert_eq!(
            item.snippet(
                "https://github.com/user-attachments/assets/video",
                ImageCopyFormat::Html
            ),
            "https://github.com/user-attachments/assets/video"
        );
    }

    #[test]
    fn rejects_empty_oversized_corrupt_and_audio_only_videos() {
        let mut file = tempfile::Builder::new().suffix(".mp4").tempfile().unwrap();
        assert!(
            StagedAttachment::from_path(file.path().into(), Default::default())
                .err()
                .unwrap()
                .to_string()
                .contains("empty")
        );
        file.as_file().set_len(MAX_ATTACHMENT_BYTES + 1).unwrap();
        assert!(
            StagedAttachment::from_path(file.path().into(), Default::default())
                .err()
                .unwrap()
                .to_string()
                .contains("10 MB")
        );
        for bytes in [
            b"not a video".to_vec(),
            VIDEO[..VIDEO.len() / 2].to_vec(),
            {
                let mut audio_only = VIDEO.to_vec();
                let position = audio_only
                    .windows(4)
                    .position(|bytes| bytes == b"vide")
                    .unwrap();
                audio_only[position..position + 4].copy_from_slice(b"soun");
                audio_only
            },
        ] {
            file.as_file().set_len(0).unwrap();
            file.rewind().unwrap();
            file.write_all(&bytes).unwrap();
            assert!(
                StagedAttachment::from_path(file.path().into(), Default::default())
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("container")
            );
        }
        assert!(validate_size(MAX_ATTACHMENT_BYTES, MAX_ATTACHMENT_BYTES).is_ok());
        assert!(validate_size(MAX_ATTACHMENT_BYTES + 1, MAX_ATTACHMENT_BYTES).is_err());
    }
}

#[cfg(test)]
mod owner_tests {
    use super::parse_owner;

    #[test]
    fn accepts_account_and_organization_logins_but_not_repository_paths() {
        assert_eq!(parse_owner(" octocat ").unwrap(), "octocat");
        assert_eq!(parse_owner("acme-labs").unwrap(), "acme-labs");
        for invalid in [
            "",
            " ",
            "https://github.com/octocat",
            "octocat/images",
            "two words",
        ] {
            assert!(parse_owner(invalid).is_err());
        }
    }
}
