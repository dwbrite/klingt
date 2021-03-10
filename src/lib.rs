pub mod nodes;

use crate::nodes::sink::CpalMonoSink;
use crate::nodes::source::{Sine, Square};

use crate::nodes::effect::SlewLimiter;
use dasp_graph::{Buffer, Input, Node};

pub enum IO {
    Sink(CpalMonoSink),
    Sine(Sine),
    Square(Square),
    Sum(dasp_graph::node::Sum),
    SlewLim(SlewLimiter),
}

impl Node for IO {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        match self {
            IO::Sink(s) => s.process(inputs, output),
            IO::Sine(s) => s.process(inputs, output),
            IO::Sum(s) => s.process(inputs, output),
            IO::SlewLim(s) => s.process(inputs, output),
            IO::Square(s) => s.process(inputs, output),
        }
    }
}

// TODO: docs, buffered sources, seeking (on seekable sources?), {mono/stereo/3d audio, hrtf, doppler}
