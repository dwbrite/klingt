//! Sine wave oscillator.

use dasp_graph::{Buffer, Input};
use crate::node::{AudioNode, ProcessContext};

/// Messages to control a [`Sine`] oscillator.
///
/// Send these via [`Handle::send`](crate::Handle::send) to change parameters at runtime.
#[derive(Clone, Copy, Debug)]
pub enum SineMessage {
    /// Set the frequency in Hz.
    SetFrequency(f32),
    /// Set the amplitude (0.0 to 1.0).
    SetAmplitude(f32),
}

/// A sine wave oscillator (mono source).
///
/// Generates a pure sine tone at the specified frequency. Default amplitude is 0.25
/// (-12dB) to prevent clipping when mixing multiple oscillators.
///
/// # Example
///
/// ```no_run
/// # use klingt::{Klingt, Handle};
/// # use klingt::nodes::{Sine, SineMessage};
/// # let mut klingt = Klingt::default_output().unwrap();
/// // Create a 440 Hz sine wave
/// let mut sine = klingt.add(Sine::new(440.0));
/// klingt.output(&sine);
///
/// // Change frequency at runtime
/// sine.send(SineMessage::SetFrequency(880.0)).ok();
/// ```
pub struct Sine {
    frequency: f32,
    phase: f32,
    amplitude: f32,
}

impl Sine {
    /// Create a new sine oscillator at the given frequency (Hz).
    ///
    /// Default amplitude is 0.25 (-12dB).
    pub fn new(frequency: f32) -> Self {
        Self {
            frequency,
            phase: 0.0,
            amplitude: 0.25, // -12dB, safe default
        }
    }

    /// Set the initial amplitude (builder pattern).
    ///
    /// Amplitude is clamped to 0.0 - 1.0.
    pub fn with_amplitude(mut self, amplitude: f32) -> Self {
        self.amplitude = amplitude.clamp(0.0, 1.0);
        self
    }

    /// Get the current frequency in Hz.
    #[inline]
    pub fn frequency(&self) -> f32 {
        self.frequency
    }

    /// Get the current amplitude.
    #[inline]
    pub fn amplitude(&self) -> f32 {
        self.amplitude
    }
}

impl AudioNode for Sine {
    type Message = SineMessage;

    fn process(
        &mut self,
        ctx: &ProcessContext,
        messages: impl Iterator<Item = SineMessage>,
        _inputs: &[Input],
        outputs: &mut [Buffer],
    ) {
        // Handle messages first
        for msg in messages {
            match msg {
                SineMessage::SetFrequency(f) => self.frequency = f.max(0.0),
                SineMessage::SetAmplitude(a) => self.amplitude = a.clamp(0.0, 1.0),
            }
        }

        if outputs.is_empty() {
            return;
        }

        let phase_inc = self.frequency / ctx.sample_rate as f32;
        let buffer_len = outputs[0].len();
        let amplitude = self.amplitude;

        // Generate samples - write to first buffer, then copy to others
        let (first, rest) = outputs.split_first_mut().unwrap();
        
        for i in 0..buffer_len {
            let sample = (self.phase * core::f32::consts::TAU).sin() * amplitude;
            first[i] = sample;

            self.phase += phase_inc;
            // Branchless phase wrap (phase is always positive)
            self.phase -= (self.phase >= 1.0) as u32 as f32;
        }

        // Copy to remaining output channels (if any)
        for buffer in rest.iter_mut() {
            buffer.copy_from_slice(first);
        }
    }

    #[inline]
    fn num_inputs(&self) -> usize { 0 }
    
    #[inline]
    fn num_outputs(&self) -> usize { 1 }
}
