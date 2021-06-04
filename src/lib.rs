pub mod nodes;

use crate::nodes::sink::{CpalMonoSink, CpalStereoSink};
use crate::nodes::source::{BufferedOgg, Sine, Square};

use crate::nodes::effect::SlewLimiter;
use dasp_graph::{Buffer, Input, Node};
// pub use enum_dispatch::enum_dispatch;

use dasp_graph::node::Sum;

#[cfg(not(feature = "custom_dispatch"))]
#[impl_enum::with_methods {
    fn process_inner(&mut self, inputs: &[Input], output: &mut [Buffer]) {}
}]
pub enum NodeVariants {
    CpalMonoSink(CpalMonoSink),
    CpalStereoSink(CpalStereoSink),
    Sine(Sine),
    Square(Square),
    Sum(Sum),
    SlewLimiter(SlewLimiter),
    BufferedOgg(BufferedOgg),
}

pub trait AudioNode {
    fn process_inner(&mut self, inputs: &[Input], output: &mut [Buffer]);
}

#[cfg(not(feature = "custom_dispatch"))]
impl Node for NodeVariants {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        self.process_inner(inputs, output);
    }
}

// TODO: docs, buffered sources, seeking (on seekable sources?), {mono/stereo/3d audio, hrtf, doppler}
