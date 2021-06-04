use crate::AudioNode;
use dasp_graph::{Buffer, Input, Node};

// TODO: find out if I'm doing myself any good by using a raw pointer instead of a smart pointer
pub struct SlewLimiter {
    channel_last: [f32; 8],
}

impl SlewLimiter {
    // TODO: parameterize sample rate, channels, calculate delta
    pub fn new() -> Self {
        SlewLimiter {
            channel_last: [0f32; 8],
        }
    }
}

impl AudioNode for SlewLimiter {
    fn process_inner(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        // Sum the inputs onto the output.
        for (channel, out_buffer) in output.iter_mut().enumerate() {
            // only accepts one input
            let input = inputs.first().unwrap();
            let in_buffers = input.buffers();
            if let Some(in_buffer) = in_buffers.get(channel) {
                for (i, o) in in_buffer.iter().zip(out_buffer.iter_mut()) {
                    // TODO: better math
                    let last = self.channel_last[channel];
                    let delta = i - last;
                    let min_per_sample = 1_f32 / 16_f32;

                    *o = last + delta.abs().min(min_per_sample).copysign(delta);
                    self.channel_last[channel] = *o;
                }
            }
        }
    }
}
