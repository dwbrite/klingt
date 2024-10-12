pub mod nodes;

use crate::nodes::sink::{CpalMonoSink, CpalStereoSink};
use crate::nodes::source::{BufferedOgg, Sine, Square};

use crate::nodes::effect::SlewLimiter;
use dasp_graph::{Buffer, Input, NodeData, Processor};
// pub use enum_dispatch::enum_dispatch;

use dasp_graph::node::Sum;

pub use dasp_graph::Node as AudioNode;

use core::convert::TryInto;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use petgraph::graph::{EdgeIndex, NodeIndex};

pub type KlingtGraph<T> = petgraph::graph::Graph<NodeData<T>, ()>;

pub struct Klingt<T: AudioNode> where {
    pub graph: KlingtGraph<T>,
    pub processor: Processor<KlingtGraph<T>>,
    pub nodes: HashMap<String, NodeIndex>,
    pub edges: HashMap<String, EdgeIndex>,
}

impl<T: AudioNode> Default for Klingt<T> {
    fn default() -> Klingt<T> {
        Klingt {
            graph: KlingtGraph::<T>::with_capacity(64, 64),
            processor: Processor::<KlingtGraph<T>>::with_capacity(64),
            nodes: Default::default(),
            edges: Default::default(),
        }
    }
}

impl <T: AudioNode> Deref for Klingt<T> {
    type Target = KlingtGraph<T>;

    fn deref(&self) -> &Self::Target {
        &self.graph
    }
}


impl <T: AudioNode> DerefMut for Klingt<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.graph
    }
}

impl <T: AudioNode> Klingt<T> {
    pub fn process_to_idx(&mut self, idx: NodeIndex) {
        self.processor.process(&mut self.graph, idx)
    }
    
    pub fn process_to_node(&mut self) {
    //     
    }
}

#[enum_delegate::implement(AudioNode,
pub trait AudioNode {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]);
}
)]
pub enum NodeVariants {
    CpalMonoSink(CpalMonoSink),
    CpalStereoSink(CpalStereoSink),
    Sine(Sine),
    Square(Square),
    Sum(Sum),
    SlewLimiter(SlewLimiter),
    BufferedOgg(BufferedOgg),
}

// TODO: docs, buffered sources, seeking (on seekable sources?), {mono/stereo/3d audio, hrtf, doppler}
