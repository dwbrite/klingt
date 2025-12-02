//! Resampling source node
//! 
//! Consumes audio from a ring buffer at one sample rate and outputs
//! at the graph's sample rate. Used to bridge graphs at different rates.

use dasp_graph::{Buffer, Input};
use rtrb::Consumer;

use crate::v2::node::{AudioNode, ProcessContext};

/// Messages for the resampling source
#[derive(Clone, Copy, Debug)]
pub enum ResamplingSourceMessage {
    /// Set the input sample rate (if it changes dynamically)
    SetInputRate(u32),
}

/// A source that reads from a ring buffer and resamples to the graph's sample rate
/// 
/// Uses linear interpolation for simplicity. For higher quality, consider
/// using rubato or a sinc interpolator.
pub struct ResamplingSource {
    consumer: Consumer<f32>,
    channels: usize,
    input_sample_rate: u32,
    
    /// Fractional position in the input stream
    position: f64,
    
    /// Buffer of recent input samples for interpolation (per channel)
    /// We keep 2 samples per channel for linear interp
    prev_samples: [f32; 16], // up to 8 channels * 2 samples
    curr_samples: [f32; 16],
    
    /// Whether we've received any samples yet
    primed: bool,
}

impl ResamplingSource {
    /// Create a resampling source
    /// 
    /// - `consumer`: Ring buffer consumer with interleaved samples at `input_sample_rate`
    /// - `channels`: Number of audio channels
    /// - `input_sample_rate`: Sample rate of the incoming audio
    pub fn new(consumer: Consumer<f32>, channels: usize, input_sample_rate: u32) -> Self {
        Self {
            consumer,
            channels: channels.min(8),
            input_sample_rate,
            position: 0.0,
            prev_samples: [0.0; 16],
            curr_samples: [0.0; 16],
            primed: false,
        }
    }

    /// Read one frame (all channels) from the ring buffer
    /// Returns true if successful
    fn read_frame(&mut self) -> bool {
        for ch in 0..self.channels {
            match self.consumer.pop() {
                Ok(sample) => self.curr_samples[ch] = sample,
                Err(_) => return false, // underrun
            }
        }
        true
    }

    /// Advance to next frame, shifting current to previous
    fn advance_frame(&mut self) {
        for ch in 0..self.channels {
            self.prev_samples[ch] = self.curr_samples[ch];
        }
    }
}

impl AudioNode for ResamplingSource {
    type Message = ResamplingSourceMessage;

    fn process(
        &mut self,
        ctx: &ProcessContext,
        messages: impl Iterator<Item = ResamplingSourceMessage>,
        _inputs: &[Input],
        outputs: &mut [Buffer],
    ) {
        // Handle messages
        for msg in messages {
            match msg {
                ResamplingSourceMessage::SetInputRate(rate) => {
                    self.input_sample_rate = rate;
                }
            }
        }

        if outputs.is_empty() {
            return;
        }

        let output_rate = ctx.sample_rate as f64;
        let input_rate = self.input_sample_rate as f64;
        let rate_ratio = input_rate / output_rate; // e.g., 48000/44100 ≈ 1.088
        
        let buffer_len = outputs[0].len();

        // Prime the interpolator if needed
        if !self.primed {
            if self.read_frame() {
                self.advance_frame();
                if self.read_frame() {
                    self.primed = true;
                }
            }
        }

        for i in 0..buffer_len {
            // Check if we need to advance to next input frame
            while self.position >= 1.0 {
                self.position -= 1.0;
                self.advance_frame();
                if !self.read_frame() {
                    // Underrun - output silence for rest of buffer
                    for buffer in outputs.iter_mut() {
                        for j in i..buffer_len {
                            buffer[j] = 0.0;
                        }
                    }
                    return;
                }
            }

            // Linear interpolation between prev and curr samples
            let t = self.position as f32;
            
            for (ch, buffer) in outputs.iter_mut().enumerate() {
                let ch_idx = ch % self.channels;
                let prev = self.prev_samples[ch_idx];
                let curr = self.curr_samples[ch_idx];
                buffer[i] = prev + t * (curr - prev);
            }

            // Advance position by the rate ratio
            self.position += rate_ratio;
        }
    }

    #[inline]
    fn num_inputs(&self) -> usize { 0 }

    #[inline]
    fn num_outputs(&self) -> usize { self.channels }
}
