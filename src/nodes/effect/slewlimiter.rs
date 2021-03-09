use dasp_graph::{Buffer, Input, Node};

// TODO: find out if I'm doing myself any good by using a raw pointer instead of a smart pointer
pub struct SlewLimiter {
    channels_last_ptr: *mut f32,
    channels_last_len: usize,
}

impl SlewLimiter {
    // TODO: parameterize sample rate, channels, calculate delta
    pub fn new() -> Self {
        SlewLimiter {
            channels_last_ptr: [0f32; 2].as_mut_ptr(),
            channels_last_len: 2,
        }
    }
}

impl Node for SlewLimiter {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        let slice = unsafe {
            std::slice::from_raw_parts_mut(self.channels_last_ptr, 4 * self.channels_last_len)
        };

        // Sum the inputs onto the output.
        for (channel, out_buffer) in output.iter_mut().enumerate() {
            // only accepts one input
            let input = inputs.first().unwrap();
            let in_buffers = input.buffers();
            if let Some(in_buffer) = in_buffers.get(channel) {
                for (i, o) in in_buffer.iter().zip(out_buffer.iter_mut()) {
                    // TODO: better math
                    let last = slice[channel];
                    let delta = i - last;

                    if delta.abs() > (1f32 / 3072_f32) {
                        *o = last + delta.abs().copysign(delta);
                    } else {
                        *o = *i;
                    }
                }
            }
        }
    }
}
