//! Mixer effect - sums multiple inputs together

use dasp_graph::{Buffer, Input};
use crate::node::{AudioNode, ProcessContext};

/// A mixer that sums multiple inputs together
/// 
/// Each input is summed with equal weight. The output has `channels` channels.
/// If an input has fewer channels, it will be upmixed (mono→stereo copies to both).
/// If an input has more channels, extra channels are ignored.
pub struct Mixer {
    channels: usize,
}

impl Mixer {
    /// Create a new mixer with the specified number of output channels
    pub fn new(channels: usize) -> Self {
        Self { channels }
    }
    
    /// Create a stereo mixer
    pub fn stereo() -> Self {
        Self::new(2)
    }
    
    /// Create a mono mixer
    pub fn mono() -> Self {
        Self::new(1)
    }
}

impl AudioNode for Mixer {
    type Message = ();
    
    fn process(
        &mut self,
        _ctx: &ProcessContext,
        _messages: impl Iterator<Item = Self::Message>,
        inputs: &[Input],
        output: &mut [Buffer],
    ) {
        // Clear output buffers
        for buf in output.iter_mut() {
            buf.iter_mut().for_each(|s| *s = 0.0);
        }
        
        // Sum all inputs
        for input in inputs {
            let input_channels = input.buffers().len();
            
            for (out_ch, out_buf) in output.iter_mut().enumerate() {
                // Determine which input channel to read from
                let in_ch = if input_channels == 1 {
                    0 // Mono input: use channel 0 for all outputs
                } else {
                    out_ch.min(input_channels - 1)
                };
                
                let in_buf = &input.buffers()[in_ch];
                for (out_sample, in_sample) in out_buf.iter_mut().zip(in_buf.iter()) {
                    *out_sample += *in_sample;
                }
            }
        }
    }
    
    fn num_inputs(&self) -> usize {
        // Accept any number of inputs
        usize::MAX
    }
    
    fn num_outputs(&self) -> usize {
        self.channels
    }
}
