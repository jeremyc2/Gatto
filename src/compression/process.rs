//! Own each helper and its pipes through completion, cancellation, or failure.
use super::CompressionControl;
use anyhow::{Context as _, Result, bail, ensure};
use std::{
    io::{BufRead as _, BufReader, Read as _},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

const OUTPUT_LIMIT: u64 = 256 * 1024;
const ERROR_LIMIT: usize = 16 * 1024;

struct Running {
    child: Child,
    stdout: Option<JoinHandle<std::io::Result<Vec<u8>>>>,
    stderr: Option<JoinHandle<std::io::Result<Vec<u8>>>>,
}

impl Drop for Running {
    fn drop(&mut self) {
        // Reap before joining readers, so every pipe reaches EOF even on cancel.
        let _ = self.child.kill();
        let _ = self.child.wait();
        for reader in [&mut self.stdout, &mut self.stderr] {
            if let Some(reader) = reader.take() {
                let _ = reader.join();
            }
        }
    }
}

pub(super) fn run(
    command: &mut Command,
    control: &CompressionControl,
    deadline: Instant,
    capture: bool,
    progress: impl Fn(u64),
) -> Result<Vec<u8>> {
    control.check()?;
    // Helpers need neither GitHub credentials nor the caller's loader/report settings.
    command
        .env_clear()
        .env("LC_ALL", "C")
        .env("TMPDIR", std::env::temp_dir())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command
        .spawn()
        .context("Could not start the bundled video encoder. Reinstall Gatto and try again.")?;
    let mut running = Running {
        child,
        stdout: None,
        stderr: None,
    };
    let stdout = running
        .child
        .stdout
        .take()
        .context("The video encoder has no output pipe")?;
    let stderr = running
        .child
        .stderr
        .take()
        .context("The video encoder has no error pipe")?;
    let overflow = Arc::new(AtomicBool::new(false));
    let exceeded = overflow.clone();
    let (sender, receiver) = mpsc::sync_channel(16);
    running.stdout = Some(
        std::thread::Builder::new()
            .name("video-progress".into())
            .spawn(move || {
                if capture {
                    let mut bytes = Vec::new();
                    stdout.take(OUTPUT_LIMIT + 1).read_to_end(&mut bytes)?;
                    exceeded.store(bytes.len() as u64 > OUTPUT_LIMIT, Ordering::Release);
                    return Ok(bytes);
                }
                let mut reader = BufReader::new(stdout);
                let mut line = Vec::new();
                loop {
                    line.clear();
                    if reader.by_ref().take(4096).read_until(b'\n', &mut line)? == 0 {
                        break;
                    }
                    if let Some(value) = line
                        .strip_prefix(b"out_time_us=")
                        .and_then(|value| std::str::from_utf8(value).ok())
                        .and_then(|value| value.trim().parse::<u64>().ok())
                    {
                        let _ = sender.try_send(value);
                    }
                }
                Ok(Vec::new())
            })
            .context("Could not read video encoding progress")?,
    );
    running.stderr = Some(
        std::thread::Builder::new()
            .name("video-errors".into())
            .spawn(move || {
                let mut reader = stderr;
                let mut tail = Vec::new();
                let mut buffer = [0; 4096];
                loop {
                    let read = reader.read(&mut buffer)?;
                    if read == 0 {
                        break;
                    }
                    tail.extend_from_slice(&buffer[..read]);
                    if tail.len() > ERROR_LIMIT {
                        tail.drain(..tail.len() - ERROR_LIMIT);
                    }
                }
                Ok(tail)
            })
            .context("Could not read video encoding errors")?,
    );
    let status = loop {
        control.check()?;
        ensure!(
            Instant::now() < deadline,
            "Video compression took too long. Try Fast mode or trim the recording."
        );
        ensure!(
            !overflow.load(Ordering::Acquire),
            "This video's metadata is too large. Export it again as an SDR MP4."
        );
        while let Ok(time) = receiver.try_recv() {
            progress(time);
        }
        if let Some(status) = running
            .child
            .try_wait()
            .context("Could not check video encoding progress")?
        {
            break status;
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    let stdout = running
        .stdout
        .take()
        .unwrap()
        .join()
        .map_err(|_| anyhow::anyhow!("Video output reader stopped unexpectedly"))??;
    let stderr = running
        .stderr
        .take()
        .unwrap()
        .join()
        .map_err(|_| anyhow::anyhow!("Video error reader stopped unexpectedly"))??;
    control.check()?;
    ensure!(
        !overflow.load(Ordering::Acquire),
        "This video's metadata is too large. Export it again as an SDR MP4."
    );
    while let Ok(time) = receiver.try_recv() {
        progress(time);
    }
    if !status.success() {
        let reason: String = String::from_utf8_lossy(&stderr)
            .trim()
            .chars()
            .take(1200)
            .collect();
        bail!("Could not compress this video. Try Fast mode or export an SDR H.264 MP4. {reason}");
    }
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_and_timeout_reap_a_running_helper_promptly() {
        for cancel in [true, false] {
            let control = CompressionControl::default();
            let started = Instant::now();
            let error = run(
                Command::new("/bin/sh")
                    .args(["-c", "printf 'out_time_us=1\\n'; exec /bin/sleep 30"]),
                &control,
                started + Duration::from_millis(150),
                false,
                |_| {
                    if cancel {
                        control.cancel();
                    }
                },
            )
            .unwrap_err();
            assert!(started.elapsed() < Duration::from_secs(2));
            if cancel {
                assert!(error.downcast_ref::<super::super::Canceled>().is_some());
            } else {
                assert!(error.to_string().contains("too long"));
            }
        }
    }
}
