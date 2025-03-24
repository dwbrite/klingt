use dasp_graph::{Buffer, Input};
use rtrb::Producer;
use crate::AudioNode;

#[derive(Debug)]
pub struct RtrbSink {
    pub output: Producer<Buffer>,
}

impl RtrbSink {
    pub fn new(output: Producer<Buffer>) -> RtrbSink {
        RtrbSink { output }
    }
}

impl AudioNode for RtrbSink {
    fn process(&mut self, inputs: &[Input], _output: &mut [Buffer]) {
        let _ = self.output.push(inputs[0].buffers()[0].clone());
    }
}

