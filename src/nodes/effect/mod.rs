mod slewlimiter;
use crate::AudioNode;
pub use dasp_graph::node::Sum;
use dasp_graph::{Buffer, Input, Node};
pub use slewlimiter::*;

impl AudioNode for Sum {
    fn process_inner(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        self.process(inputs, output);
    }
}
