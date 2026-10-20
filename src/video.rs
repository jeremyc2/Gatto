//! Bounded video snapshots and container checks. Playback/codec validation belongs
//! to the macOS video application; container validation never transcodes media.
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    process::{Command, Stdio},
};

use anyhow::{Context as _, Result, ensure};
use tempfile::NamedTempFile;

use crate::model::{VideoUploadLimit, validate_size};

pub struct Video {
    file: NamedTempFile,
    len: u64,
}

impl Video {
    pub fn mime_type(extension: &str) -> Option<&'static str> {
        match extension {
            "mp4" => Some("video/mp4"),
            "mov" => Some("video/quicktime"),
            "webm" => Some("video/webm"),
            _ => None,
        }
    }

    pub fn stage(source: &mut File, extension: &str, limit: VideoUploadLimit) -> Result<Self> {
        let mut file = tempfile::Builder::new()
            .prefix("gatto-video-")
            .suffix(&format!(".{extension}"))
            .tempfile()
            .context(
                "Could not create a private temporary video copy. Check available disk space.",
            )?;
        let len = std::io::copy(&mut source.take(limit.bytes() + 1), file.as_file_mut())
            .context("Could not copy the video. Check file access and available disk space.")?;
        validate_size(len, limit.bytes())?;
        file.flush()
            .context("Could not finish the temporary video copy")?;
        file.rewind()?;
        let valid = if extension == "webm" {
            validate_webm(file.as_file_mut(), len)
        } else {
            validate_mp4(file.as_file_mut(), len)
        };
        valid.context("The video container is invalid, incomplete, or has no video track. Export it again as H.264 MP4 and try again.")?;
        Ok(Self { file, len })
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn reader(&self) -> Result<File> {
        self.file
            .reopen()
            .context("The staged video copy is unavailable. Add the video again.")
    }

    /// Run on a background task that owns an Arc<Video>. -W retains the snapshot
    /// while the external viewer is using it, even after the draft is removed.
    pub fn preview(&self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            let status = Command::new("/usr/bin/open")
                .arg("-W")
                .arg(self.file.path())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .context("Could not launch the macOS video application")?;
            ensure!(
                status.success(),
                "macOS could not open this video. Install an app that supports its format, or export it as H.264 MP4 and try again."
            );
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        anyhow::bail!("Video preview currently requires macOS.")
    }
}

// ISO BMFF/QuickTime top-level boxes, with video handler validation inside moov.
// Skips media payloads rather than loading them into memory.
fn validate_mp4(file: &mut File, len: u64) -> Result<()> {
    let mut position = 0;
    let mut media = false;
    let mut video = false;
    while position < len {
        let (kind, start, end) = mp4_box(file, position, len)?;
        if kind == *b"mdat" {
            media |= end > start;
        }
        if kind == *b"moov" {
            video |= has_video_handler(file, start, end, 0)?;
        }
        position = end;
    }
    ensure!(media && video, "Missing video track or media payload");
    Ok(())
}

fn mp4_box(file: &mut File, position: u64, limit: u64) -> Result<([u8; 4], u64, u64)> {
    ensure!(limit.saturating_sub(position) >= 8, "Truncated MP4 box");
    file.seek(SeekFrom::Start(position))?;
    let mut header = [0; 8];
    file.read_exact(&mut header)?;
    let size = u32::from_be_bytes(header[..4].try_into()?) as u64;
    let (size, header_len) = match size {
        0 => (limit - position, 8),
        1 => {
            let mut extended = [0; 8];
            file.read_exact(&mut extended)?;
            (u64::from_be_bytes(extended), 16)
        }
        value => (value, 8),
    };
    ensure!(
        size >= header_len && size <= limit - position,
        "Invalid MP4 box size"
    );
    Ok((
        header[4..8].try_into()?,
        position + header_len,
        position + size,
    ))
}

fn has_video_handler(file: &mut File, mut position: u64, limit: u64, depth: u8) -> Result<bool> {
    ensure!(depth < 4, "Invalid MP4 nesting");
    let mut video = false;
    while position < limit {
        let (kind, start, end) = mp4_box(file, position, limit)?;
        if matches!(&kind, b"trak" | b"mdia") {
            video |= has_video_handler(file, start, end, depth + 1)?;
        } else if kind == *b"hdlr" {
            ensure!(end - start >= 12, "Truncated video handler");
            file.seek(SeekFrom::Start(start + 8))?;
            let mut handler = [0; 4];
            file.read_exact(&mut handler)?;
            video |= handler == *b"vide";
        }
        position = end;
    }
    Ok(video)
}

// Read EBML variable integers without allocating element-sized buffers.
fn vint(file: &mut File, keep_marker: bool) -> Result<Option<u64>> {
    let mut first = [0];
    file.read_exact(&mut first)?;
    let width = first[0].leading_zeros() as usize + 1;
    ensure!(width <= 8, "Invalid EBML integer");
    let marker = 1_u8 << (8 - width);
    let mut value = if keep_marker {
        first[0]
    } else {
        first[0] & (marker - 1)
    } as u64;
    for _ in 1..width {
        let mut next = [0];
        file.read_exact(&mut next)?;
        value = (value << 8) | next[0] as u64;
    }
    Ok(if !keep_marker && value == (1_u64 << (7 * width)) - 1 {
        None
    } else {
        Some(value)
    })
}

fn ebml_element(file: &mut File, position: u64, limit: u64) -> Result<(u64, u64, u64)> {
    file.seek(SeekFrom::Start(position))?;
    let id = vint(file, true)?.context("Missing EBML ID")?;
    let size = vint(file, false)?;
    let start = file.stream_position()?;
    ensure!(start <= limit, "Truncated EBML element");
    let size = size.unwrap_or(limit - start);
    ensure!(size <= limit - start, "Invalid EBML size");
    Ok((id, start, start + size))
}

fn validate_webm(file: &mut File, len: u64) -> Result<()> {
    let (id, mut position, header_end) = ebml_element(file, 0, len)?;
    ensure!(id == 0x1a45dfa3, "Missing EBML header");
    let mut webm = false;
    while position < header_end {
        let (id, start, end) = ebml_element(file, position, header_end)?;
        if id == 0x4282 && end - start == 4 {
            file.seek(SeekFrom::Start(start))?;
            let mut doc_type = [0; 4];
            file.read_exact(&mut doc_type)?;
            webm = doc_type == *b"webm";
        }
        position = end;
    }
    ensure!(webm, "Not a WebM document");
    position = header_end;
    let mut video = false;
    let mut media = false;
    while position < len {
        let (id, start, end) = ebml_element(file, position, len)?;
        if id == 0x18538067 {
            let mut child = start;
            while child < end {
                let (id, start, next) = ebml_element(file, child, end)?;
                if id == 0x1654ae6b {
                    video |= webm_video_track(file, start, next)?;
                }
                if id == 0x1f43b675 {
                    media |= next > start;
                }
                child = next;
            }
        }
        position = end;
    }
    ensure!(video && media, "Missing WebM video track or media payload");
    Ok(())
}

fn webm_video_track(file: &mut File, mut position: u64, limit: u64) -> Result<bool> {
    let mut video = false;
    while position < limit {
        let (id, start, end) = ebml_element(file, position, limit)?;
        if id == 0xae {
            let mut child = start;
            while child < end {
                let (id, start, next) = ebml_element(file, child, end)?;
                if id == 0x83 && next - start == 1 {
                    file.seek(SeekFrom::Start(start))?;
                    let mut kind = [0];
                    file.read_exact(&mut kind)?;
                    video |= kind[0] == 1;
                }
                child = next;
            }
        }
        position = end;
    }
    Ok(video)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_webm_tracks_and_rejects_truncated_elements() {
        // Minimal structural fixture: EBML DocType=webm, a video TrackEntry,
        // and a Cluster. Payload decoding is the external player's concern.
        let bytes = [
            0x1a, 0x45, 0xdf, 0xa3, 0x87, 0x42, 0x82, 0x84, b'w', b'e', b'b', b'm', 0x18, 0x53,
            0x80, 0x67, 0x90, 0x16, 0x54, 0xae, 0x6b, 0x85, 0xae, 0x83, 0x83, 0x81, 0x01, 0x1f,
            0x43, 0xb6, 0x75, 0x81, 0x00,
        ];
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(&bytes).unwrap();
        assert!(validate_webm(&mut file, bytes.len() as u64).is_ok());
        assert!(validate_webm(&mut file, bytes.len() as u64 - 1).is_err());
        file.seek(SeekFrom::Start(26)).unwrap();
        file.write_all(&[2]).unwrap(); // Change video track to audio.
        assert!(validate_webm(&mut file, bytes.len() as u64).is_err());
    }

    #[test]
    fn malformed_box_sizes_and_ebml_lengths_terminate_with_errors() {
        for bytes in [
            vec![0; 7],
            vec![0, 0, 0, 1, b'm', b'o', b'o', b'v'],
            vec![0, 0, 0, 4, b'm', b'd', b'a', b't'],
        ] {
            let mut file = tempfile::tempfile().unwrap();
            file.write_all(&bytes).unwrap();
            assert!(validate_mp4(&mut file, bytes.len() as u64).is_err());
        }
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(&[0; 16]).unwrap();
        assert!(validate_webm(&mut file, 16).is_err());
    }

    #[test]
    fn private_snapshot_is_removed_when_last_owner_is_released() {
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(include_bytes!("../tests/fixtures/sample.mp4"))
            .unwrap();
        file.rewind().unwrap();
        let video =
            std::sync::Arc::new(Video::stage(&mut file, "mp4", Default::default()).unwrap());
        let path = video.file.path().to_owned();
        let viewer = video.clone();
        drop(video);
        assert!(path.exists());
        drop(viewer);
        assert!(!path.exists());
    }
}
