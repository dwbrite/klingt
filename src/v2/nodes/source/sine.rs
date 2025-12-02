//! Sine wave oscillator

use dasp_graph::{Buffer, Input};
use crate::v2::node::{AudioNode, ProcessContext};

/// Messages to control a Sine oscillator
#[derive(Clone, Copy, Debug)]
pub enum SineMessage {
    SetFrequency(f32),
    SetAmplitude(f32),
}

/// A sine wave oscillator (mono source)
pub struct Sine {
    frequency: f32,
    phase: f32,
    amplitude: f32,
}

impl Sine {
    pub fn new(frequency: f32) -> Self {
        Self {
            frequency,
            phase: 0.0,
            amplitude: 0.25, // -12dB, safe default
        }
    }

    pub fn with_amplitude(mut self, amplitude: f32) -> Self {
        self.amplitude = amplitude.clamp(0.0, 1.0);
        self
    }

    #[inline]
    pub fn frequency(&self) -> f32 {
        self.frequency
    }

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
