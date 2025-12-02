//! Slew rate limiter effect

use dasp_graph::{Buffer, Input};
use crate::v2::node::{AudioNode, ProcessContext};

/// Messages to control the slew limiter
#[derive(Clone, Copy, Debug)]
pub enum SlewLimiterMessage {
    /// Set the maximum rate of change per sample (0.0 - 1.0)
    SetRate(f32),
    /// Set rate in units per second (will be converted based on sample rate)
    SetRatePerSecond(f32),
}

/// A slew rate limiter that smooths sudden changes in audio
/// 
/// Useful for:
/// - Smoothing control signals
/// - Creating portamento/glide effects
/// - Reducing harsh transients
pub struct SlewLimiter {
    /// Maximum change per sample
    rate: f32,
    /// Last output value per channel (up to 8 channels)
    last: [f32; 8],
    /// Cached rate per second for recalculation
    rate_per_second: Option<f32>,
}

impl SlewLimiter {
    /// Create a new slew limiter with the given rate per sample
    /// 
    /// A rate of 1.0 means the signal can change by at most 1.0 per sample.
    /// Lower values = more smoothing.
    pub fn new(rate: f32) -> Self {
        Self {
            rate: rate.abs(),
            last: [0.0; 8],
            rate_per_second: None,
        }
    }

    /// Create with a rate specified in units per second
    /// 
    /// For example, `from_rate_per_second(1000.0)` at 48kHz means
    /// the signal can change by ~0.02 per sample.
    pub fn from_rate_per_second(rate: f32) -> Self {
        Self {
            rate: 0.0, // Will be set on first process
            last: [0.0; 8],
            rate_per_second: Some(rate),
        }
    }

    #[inline]
    pub fn rate(&self) -> f32 {
        self.rate
    }
}

impl AudioNode for SlewLimiter {
    type Message = SlewLimiterMessage;

    fn process(
        &mut self,
        ctx: &ProcessContext,
        messages: impl Iterator<Item = SlewLimiterMessage>,
        inputs: &[Input],
        outputs: &mut [Buffer],
    ) {
        // Handle messages
        for msg in messages {
            match msg {
                SlewLimiterMessage::SetRate(r) => {
                    self.rate = r.abs();
                    self.rate_per_second = None;
                }
                SlewLimiterMessage::SetRatePerSecond(r) => {
                    self.rate_per_second = Some(r);
                    self.rate = r / ctx.sample_rate as f32;
                }
            }
        }

        // Convert rate_per_second on first run if needed
        if let Some(rps) = self.rate_per_second {
            self.rate = rps / ctx.sample_rate as f32;
        }

        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let input = &inputs[0];
        let in_buffers = input.buffers();

        if in_buffers.is_empty() {
            for buffer in outputs.iter_mut() {
                buffer.iter_mut().for_each(|s| *s = 0.0);
            }
            return;
        }

        let max_delta = self.rate;

        for (ch, out_buffer) in outputs.iter_mut().enumerate() {
            let in_buffer = in_buffers.get(ch).unwrap_or_else(|| in_buffers.last().unwrap());
            let mut last = self.last[ch.min(7)];

            for (out_sample, &in_sample) in out_buffer.iter_mut().zip(in_buffer.iter()) {
                let delta = in_sample - last;
                // Clamp delta to max rate
                let clamped_delta = delta.clamp(-max_delta, max_delta);
                last += clamped_delta;
                *out_sample = last;
            }

            self.last[ch.min(7)] = last;
        }
    }

    #[inline]
    fn num_inputs(&self) -> usize { 1 }

    #[inline]
    fn num_outputs(&self) -> usize { 2 }
}
