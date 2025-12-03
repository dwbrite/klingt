//! Audio sample player.

use alloc::vec::Vec;
use dasp_graph::{Buffer, Input};
use crate::node::{AudioNode, ProcessContext};

/// Messages to control a [`SamplePlayer`].
///
/// Send these via [`Handle::send`](crate::Handle::send) to control playback.
#[derive(Clone, Copy, Debug)]
pub enum PlayerMessage {
    /// Start or resume playback.
    Play,
    /// Pause playback (keeps position).
    Pause,
    /// Stop playback and reset to beginning.
    Stop,
    /// Set playback volume (0.0 to 2.0, where 1.0 is unity gain).
    SetVolume(f32),
    /// Seek to position in seconds.
    Seek(f64),
    /// Enable or disable looping.
    SetLooping(bool),
}

/// Plays pre-decoded audio samples.
///
/// This node plays audio that's already been decoded into memory as f32 samples.
/// It reports its native sample rate, so Klingt will automatically handle
/// resampling if the output device runs at a different rate.
///
/// # Example
///
/// ```ignore
/// use symphonium::SymphoniumLoader;
/// use klingt::{Klingt, nodes::SamplePlayer};
///
/// let decoded = SymphoniumLoader::new().load_f32("music.ogg", None)?;
/// let mut player = SamplePlayer::new(
///     decoded.as_interleaved(),
///     decoded.channels(),
///     decoded.sample_rate,
/// );
/// player.set_looping(true);
///
/// let mut klingt = Klingt::default_output()?;
/// let handle = klingt.add(player);
/// klingt.output(&handle);
/// ```
///
/// # Large Files
///
/// For very large files, loading everything into memory may not be ideal.
/// Consider implementing a streaming player using a ring buffer fed by
/// a decoder thread.
pub struct SamplePlayer {
    samples: Vec<f32>,
    channels: usize,
    sample_rate: u32,
    position: usize,
    playing: bool,
    volume: f32,
    looping: bool,
}

impl SamplePlayer {
    /// Create a player from interleaved audio samples.
    ///
    /// # Arguments
    ///
    /// - `samples` - Interleaved audio data (L, R, L, R, ... for stereo)
    /// - `channels` - Number of channels in the audio data
    /// - `sample_rate` - Sample rate of the audio data in Hz
    ///
    /// Playback starts immediately. Use [`PlayerMessage::Pause`] to start paused.
    pub fn new(samples: Vec<f32>, channels: usize, sample_rate: u32) -> Self {
        Self {
            samples,
            channels: channels.max(1),
            sample_rate,
            position: 0,
            playing: true,
            volume: 1.0,
            looping: false,
        }
    }

    /// Enable or disable looping.
    ///
    /// When enabled, playback restarts from the beginning when it reaches the end.
    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    /// Get the source sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the number of audio channels.
    #[inline]
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Get the total duration in seconds.
    #[inline]
    pub fn duration_secs(&self) -> f64 {
        (self.samples.len() / self.channels) as f64 / self.sample_rate as f64
    }

    /// Get the current playback position in seconds.
    #[inline]
    pub fn position_secs(&self) -> f64 {
        (self.position / self.channels) as f64 / self.sample_rate as f64
    }

    /// Check if playback is currently active.
    #[inline]
    pub fn is_playing(&self) -> bool {
        self.playing
    }
}

impl AudioNode for SamplePlayer {
    type Message = PlayerMessage;

    fn process(
        &mut self,
        _ctx: &ProcessContext,
        messages: impl Iterator<Item = PlayerMessage>,
        _inputs: &[Input],
        outputs: &mut [Buffer],
    ) {
        // Handle messages
        for msg in messages {
            match msg {
                PlayerMessage::Play => self.playing = true,
                PlayerMessage::Pause => self.playing = false,
                PlayerMessage::Stop => {
                    self.playing = false;
                    self.position = 0;
                }
                PlayerMessage::SetVolume(v) => self.volume = v.clamp(0.0, 2.0),
                PlayerMessage::Seek(secs) => {
                    let frame = (secs * self.sample_rate as f64) as usize;
                    let sample_pos = frame * self.channels;
                    self.position = sample_pos.min(self.samples.len());
                }
                PlayerMessage::SetLooping(l) => self.looping = l,
            }
        }

        if outputs.is_empty() {
            return;
        }

        let buffer_len = outputs[0].len();

        // Fast path: not playing - output silence
        if !self.playing {
            for buffer in outputs.iter_mut() {
                buffer.iter_mut().for_each(|s| *s = 0.0);
            }
            return;
        }

        let volume = self.volume;
        let src_channels = self.channels;
        let total_samples = self.samples.len();

        for i in 0..buffer_len {
            // Check for end of samples
            if self.position >= total_samples {
                if self.looping {
                    self.position = 0;
                } else {
                    // Fill remaining with silence
                    for buffer in outputs.iter_mut() {
                        for j in i..buffer_len {
                            buffer[j] = 0.0;
                        }
                    }
                    self.playing = false;
                    return;
                }
            }

            // Write each output channel
            for (ch, buffer) in outputs.iter_mut().enumerate() {
                // Map output channel to source channel (wrap if more outputs than source)
                let src_ch = ch % src_channels;
                let sample_idx = self.position + src_ch;

                buffer[i] = if sample_idx < total_samples {
                    // Safety: we checked bounds above
                    unsafe { *self.samples.get_unchecked(sample_idx) * volume }
                } else {
                    0.0
                };
            }

            // Advance by one frame (all channels)
            self.position += src_channels;
        }
    }

    #[inline]
    fn num_inputs(&self) -> usize { 0 }

    #[inline]
    fn num_outputs(&self) -> usize {
        self.channels
    }

    #[inline]
    fn native_sample_rate(&self) -> Option<u32> {
        Some(self.sample_rate)
    }
}
