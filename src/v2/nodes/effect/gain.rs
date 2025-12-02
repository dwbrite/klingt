//! Gain/volume control effect

use dasp_graph::{Buffer, Input};
use crate::v2::node::{AudioNode, ProcessContext};

/// Messages to control gain
#[derive(Clone, Copy, Debug)]
pub enum GainMessage {
    /// Set the gain multiplier (1.0 = unity, 0.0 = silence)
    SetGain(f32),
}

/// A gain (volume) control that passes audio through with amplitude scaling
/// 
/// Supports any number of channels - each input channel maps to corresponding output.
pub struct Gain {
    gain: f32,
    /// Smoothing to prevent clicks on rapid gain changes
    smoothed_gain: f32,
    /// Smoothing coefficient (0.0 = instant, 1.0 = no change)
    smooth_coeff: f32,
}

impl Gain {
    /// Create a new gain node with the specified gain value
    pub fn new(gain: f32) -> Self {
        Self {
            gain,
            smoothed_gain: gain,
            smooth_coeff: 0.995, // ~7ms at 48kHz
        }
    }

    /// Set the smoothing time in milliseconds
    pub fn with_smoothing_ms(mut self, ms: f32, sample_rate: u32) -> Self {
        // Time constant: after `ms` milliseconds, we've reached ~63% of target
        let samples = (ms / 1000.0) * sample_rate as f32;
        self.smooth_coeff = (-1.0 / samples).exp();
        self
    }

    /// Disable smoothing for instant gain changes
    pub fn without_smoothing(mut self) -> Self {
        self.smooth_coeff = 0.0;
        self
    }

    #[inline]
    pub fn gain(&self) -> f32 {
        self.gain
    }
}

impl AudioNode for Gain {
    type Message = GainMessage;

    fn process(
        &mut self,
        _ctx: &ProcessContext,
        messages: impl Iterator<Item = GainMessage>,
        inputs: &[Input],
        outputs: &mut [Buffer],
    ) {
        // Update target gain from messages
        for msg in messages {
            match msg {
                GainMessage::SetGain(g) => self.gain = g,
            }
        }

        if inputs.is_empty() || outputs.is_empty() {
            return;
        }

        let input = &inputs[0];
        let in_buffers = input.buffers();

        if in_buffers.is_empty() {
            // No input buffers - output silence
            for buffer in outputs.iter_mut() {
                buffer.iter_mut().for_each(|s| *s = 0.0);
            }
            return;
        }

        let smooth_coeff = self.smooth_coeff;
        let target_gain = self.gain;
        let mut current_gain = self.smoothed_gain;

        // Process each output channel
        for (ch, out_buffer) in outputs.iter_mut().enumerate() {
            // Get input for this channel, or last available channel
            let in_buffer = in_buffers.get(ch).unwrap_or_else(|| in_buffers.last().unwrap());

            // Reset smoothed gain for each channel (they should track together)
            let mut gain = current_gain;

            for (out_sample, &in_sample) in out_buffer.iter_mut().zip(in_buffer.iter()) {
                // Apply smoothing: gain moves toward target
                gain = target_gain + smooth_coeff * (gain - target_gain);
                *out_sample = in_sample * gain;
            }

            // Only update the stored value once (from first channel)
            if ch == 0 {
                current_gain = gain;
            }
        }

        self.smoothed_gain = current_gain;
    }

    #[inline]
    fn num_inputs(&self) -> usize { 1 }

    #[inline]
    fn num_outputs(&self) -> usize { 2 } // Stereo pass-through by default
}
