//! Shared-mode WASAPI playback. The owner thread keeps COM, the endpoint and
//! its render buffer on the same thread. The OS handles hardware conversion.

use super::{
    Controls, new_synth,
    synth::{SAMPLE_RATE, Synth},
};
use ::windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        Media::Audio::{
            AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
            IAudioClient, IAudioRenderClient, IMMDeviceEnumerator, MMDeviceEnumerator,
            WAVEFORMATEX, eMultimedia, eRender,
        },
        System::{
            Com::{
                CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
            },
            Threading::{CreateEventW, WaitForSingleObject},
        },
    },
    core::{Error, PCWSTR},
};
use anyhow::{Context as _, Result, ensure};
use std::{
    sync::{
        Arc,
        atomic::Ordering,
        mpsc::{Receiver, RecvTimeoutError},
    },
    time::{Duration, Instant},
};

struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: The owner balances its successful initialization.
        unsafe { CoUninitialize() };
    }
}

struct RenderEvent(HANDLE);
impl Drop for RenderEvent {
    fn drop(&mut self) {
        // SAFETY: The stream has already released its reference to the event.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

struct Output {
    // Fields drop in this order so WASAPI releases before the event and COM.
    render: IAudioRenderClient,
    client: IAudioClient,
    event: RenderEvent,
    frames: u32,
    _apartment: Apartment,
}

impl Output {
    fn open() -> Result<Self> {
        // SAFETY: These interfaces are created, used and released on this
        // thread. Shared mode copies the explicit stereo format on Initialize.
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            let apartment = Apartment;
            let event = RenderEvent(CreateEventW(None, false, false, PCWSTR::null())?);
            let devices: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let device = devices
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .context("Could not find the Windows default audio endpoint")?;
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
            let format = WAVEFORMATEX {
                wFormatTag: 3, // WAVE_FORMAT_IEEE_FLOAT
                nChannels: 2,
                nSamplesPerSec: SAMPLE_RATE as u32,
                nAvgBytesPerSec: SAMPLE_RATE as u32 * 8,
                nBlockAlign: 8,
                wBitsPerSample: 32,
                cbSize: 0,
            };
            client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_EVENTCALLBACK
                        | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
                        | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                    0,
                    0,
                    &format,
                    None,
                )
                .context("Could not initialize Windows audio playback")?;
            client.SetEventHandle(event.0)?;
            let frames = client.GetBufferSize()?;
            let render: IAudioRenderClient = client.GetService()?;
            Ok(Self {
                render,
                client,
                event,
                frames,
                _apartment: apartment,
            })
        }
    }

    fn start(&self, controls: &Controls, synth: &mut Synth) -> Result<()> {
        self.fill(controls, synth)?;
        // SAFETY: The initialized stream has its event registered and buffer primed.
        unsafe { self.client.Start()? };
        Ok(())
    }

    fn wait(&self) -> Result<()> {
        // SAFETY: The event remains open for the duration of the wait.
        match unsafe { WaitForSingleObject(self.event.0, 120) } {
            WAIT_OBJECT_0 | WAIT_TIMEOUT => Ok(()),
            _ => Err(Error::from_thread().into()),
        }
    }

    fn fill(&self, controls: &Controls, synth: &mut Synth) -> Result<()> {
        // SAFETY: GetBuffer lends frames in the format supplied to Initialize.
        // The buffer stays exclusively borrowed until the matching ReleaseBuffer.
        unsafe {
            let frames = self.frames.saturating_sub(self.client.GetCurrentPadding()?);
            if frames == 0 {
                return Ok(());
            }
            let packet = self.render.GetBuffer(frames)?;
            if packet.is_null() {
                self.render.ReleaseBuffer(0, 0)?;
                ensure!(!packet.is_null(), "Windows returned an empty audio packet");
            }
            let stereo = packet.cast::<[f32; 2]>();
            controls.render(synth, frames as usize, |index, samples| {
                stereo.add(index).write(samples)
            });
            self.render.ReleaseBuffer(frames, 0)?;
        }
        Ok(())
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        // SAFETY: The endpoint and all interfaces still belong to this owner.
        let _ = unsafe { self.client.Stop() };
    }
}

pub(super) fn worker(
    controls: Arc<Controls>,
    receiver: Receiver<()>,
    events: async_channel::Sender<()>,
) {
    let mut synth = new_synth();
    let mut output: Option<Output> = None;
    let mut epoch = controls.epoch.load(Ordering::Acquire);
    let mut retry = controls.retry.load(Ordering::Acquire);
    let mut retry_at = Instant::now();
    let mut last_active = Instant::now();
    let tail = Duration::from_millis(80);

    while !controls.shutdown.load(Ordering::Acquire) {
        let now = Instant::now();
        let current_epoch = controls.epoch.load(Ordering::Acquire);
        if current_epoch != epoch {
            output = None;
            synth = new_synth();
            epoch = current_epoch;
            retry_at = now;
        }
        let current_retry = controls.retry.load(Ordering::Acquire);
        if current_retry != retry {
            retry = current_retry;
            retry_at = now;
        }
        let active = controls.active.load(Ordering::Acquire)
            || controls.preview.load(Ordering::Acquire) > 1
            || controls.previewing.load(Ordering::Acquire);
        if active {
            last_active = now;
        } else if now.duration_since(last_active) >= tail {
            output = None;
        }

        let rendered = if let Some(stream) = &output {
            stream
                .wait()
                .and_then(|()| stream.fill(&controls, &mut synth))
        } else if active && now >= retry_at {
            Output::open().and_then(|stream| {
                stream.start(&controls, &mut synth)?;
                output = Some(stream);
                if controls.unavailable.swap(false, Ordering::AcqRel) {
                    let _ = events.try_send(());
                }
                Ok(())
            })
        } else {
            let disconnected = if active {
                receiver.recv_timeout(retry_at.saturating_duration_since(now))
                    == Err(RecvTimeoutError::Disconnected)
            } else {
                receiver.recv().is_err()
            };
            if disconnected {
                break;
            }
            continue;
        };

        if let Err(error) = rendered {
            output = None;
            retry_at = Instant::now() + Duration::from_secs(2);
            controls.preview.store(0, Ordering::Release);
            controls.previewing.store(false, Ordering::Release);
            if !controls.unavailable.swap(true, Ordering::AcqRel) {
                eprintln!("Rooftops audio unavailable: {error:#}");
                let _ = events.try_send(());
            }
        }
    }
}
