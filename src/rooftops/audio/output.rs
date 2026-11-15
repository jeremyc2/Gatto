use super::{Controls, new_synth, synth::SAMPLE_RATE};
use anyhow::{Context as _, Result};
use block2::RcBlock;
use objc2::{AnyThread as _, rc::Retained, runtime::Bool};
use objc2_avf_audio::{AVAudioEngine, AVAudioFormat, AVAudioSourceNode};
use objc2_core_audio_types::{AudioBufferList, AudioTimeStamp};
use std::{
    ptr::NonNull,
    sync::{Arc, Mutex, atomic::Ordering},
};

pub(super) struct Output {
    engine: Retained<AVAudioEngine>,
    source: Retained<AVAudioSourceNode>,
}

impl Output {
    pub(super) fn new(controls: Arc<Controls>) -> Result<Self> {
        let synth = Mutex::new(new_synth());
        let render = RcBlock::new(
            move |mut silent: NonNull<Bool>,
                  _: NonNull<AudioTimeStamp>,
                  frames: u32,
                  data: NonNull<AudioBufferList>|
                  -> i32 {
                // The explicit source format below is planar Float32 stereo. AVF
                // supplies writable, frame-sized buffers and performs any hardware
                // conversion. Validate that contract before making Rust slices.
                unsafe {
                    let list = data.as_ptr();
                    if (*list).mNumberBuffers != 2 {
                        controls.render_errors.fetch_add(1, Ordering::Relaxed);
                        return -50;
                    }
                    let buffers = std::slice::from_raw_parts_mut(
                        std::ptr::addr_of_mut!((*list).mBuffers)
                            .cast::<objc2_core_audio_types::AudioBuffer>(),
                        2,
                    );
                    if buffers.iter().any(|buffer| {
                        buffer.mData.is_null()
                            || buffer.mNumberChannels != 1
                            || buffer.mDataByteSize < frames.saturating_mul(4)
                    }) {
                        controls.render_errors.fetch_add(1, Ordering::Relaxed);
                        return -50;
                    }
                    let left = std::slice::from_raw_parts_mut(
                        buffers[0].mData.cast::<f32>(),
                        frames as usize,
                    );
                    let right = std::slice::from_raw_parts_mut(
                        buffers[1].mData.cast::<f32>(),
                        frames as usize,
                    );
                    // Never wait for a lock on the render thread. Only this block
                    // accesses the score; a contended/poisoned callback emits silence.
                    let Ok(mut synth) = synth.try_lock() else {
                        left.fill(0.);
                        right.fill(0.);
                        *silent.as_mut() = Bool::YES;
                        return 0;
                    };
                    let has_signal =
                        controls.render(&mut synth, frames as usize, |index, frame| {
                            left[index] = frame[0];
                            right[index] = frame[1];
                        });
                    *silent.as_mut() = Bool::new(!has_signal);
                }
                0
            },
        );
        // AVAudioSourceNode copies the block. The engine owns the attached
        // source until Drop stops and detaches it; no borrowed Rust data escapes.
        unsafe {
            let format = AVAudioFormat::initStandardFormatWithSampleRate_channels(
                AVAudioFormat::alloc(),
                f64::from(SAMPLE_RATE),
                2,
            )
            .context("Could not create the Rooftops audio format")?;
            let source = AVAudioSourceNode::initWithFormat_renderBlock(
                AVAudioSourceNode::alloc(),
                &format,
                RcBlock::as_ptr(&render),
            );
            let engine = AVAudioEngine::init(AVAudioEngine::alloc());
            engine.attachNode(&source);
            engine.connect_to_format(&source, &engine.mainMixerNode(), Some(&format));
            engine.prepare();
            engine
                .startAndReturnError()
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            Ok(Self { engine, source })
        }
    }

    pub(super) fn ensure_running(&self) -> Result<()> {
        // Device changes may stop AVF's graph. The source format stays fixed;
        // the main mixer negotiates the new hardware format on restart.
        unsafe {
            if !self.engine.isRunning() {
                self.engine
                    .startAndReturnError()
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            }
        }
        Ok(())
    }

    pub(super) fn pause(&self) {
        // Called on the audio owner after the render block's fade has finished.
        unsafe {
            self.engine.pause();
        }
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        // Stop callback delivery before releasing the node and its render block.
        unsafe {
            self.engine.stop();
            self.engine.detachNode(&self.source);
        }
    }
}
