//! Ring buffer sink for custom audio processing

use dasp_graph::{Buffer, Input};
use rtrb::Producer;

use crate::node::{AudioNode, ProcessContext};

/// A sink that pushes audio into an rtrb ring buffer
/// 
/// Useful for:
/// - Custom audio processing pipelines
/// - Sending audio to another thread
/// - Recording/analysis
pub struct RtrbSink {
    producer: Producer<f32>,
    channels: usize,
}

impl RtrbSink {
    /// Create a sink that writes interleaved samples to the given producer
    pub fn new(producer: Producer<f32>, channels: usize) -> Self {
        Self {
            producer,
            channels: channels.max(1),
        }
    }

    /// Create a sink for mono audio
    pub fn mono(producer: Producer<f32>) -> Self {
        Self::new(producer, 1)
    }

    /// Create a sink for stereo audio
    pub fn stereo(producer: Producer<f32>) -> Self {
        Self::new(producer, 2)
    }

    /// Returns how many sample slots are available
    #[inline]
    pub fn available(&self) -> usize {
        self.producer.slots()
    }
}

impl AudioNode for RtrbSink {
    type Message = (); // No control messages

    fn process(
        &mut self,
        _ctx: &ProcessContext,
        _messages: impl Iterator<Item = ()>,
        inputs: &[Input],
        _outputs: &mut [Buffer],
    ) {
        if inputs.is_empty() {
            return;
        }

        let input = &inputs[0];
        let buffers = input.buffers();

        if buffers.is_empty() {
            return;
        }

        let buffer_len = buffers[0].len();
        let samples_needed = buffer_len * self.channels;

        // Skip if buffer is full
        if self.producer.slots() < samples_needed {
            return;
        }

        // Interleave channels
        for i in 0..buffer_len {
            for ch in 0..self.channels {
                let src_ch = ch.min(buffers.len() - 1);
                let _ = self.producer.push(buffers[src_ch][i]);
            }
        }
    }

    #[inline]
    fn num_inputs(&self) -> usize { 1 }

    #[inline]
    fn num_outputs(&self) -> usize { 0 }
}
