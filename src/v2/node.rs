//! Core node trait and types

use dasp_graph::{Buffer, Input};

/// Context available during audio processing
#[derive(Clone, Copy, Debug)]
pub struct ProcessContext {
    pub sample_rate: u32,
    pub buffer_size: usize,
}

/// Unique identifier for a node within a graph
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(pub(crate) u32);

/// The core trait for audio processing nodes
///
/// Nodes receive parameter updates via an iterator of messages,
/// processed at the start of each audio block.
pub trait AudioNode: Send + 'static {
    /// Message type for parameter updates (use `()` if none needed)
    type Message: Send + 'static;

    /// Process one block of audio
    ///
    /// 1. Drain and handle all pending messages
    /// 2. Read from inputs, write to outputs
    fn process(
        &mut self,
        ctx: &ProcessContext,
        messages: impl Iterator<Item = Self::Message>,
        inputs: &[Input],
        outputs: &mut [Buffer],
    );

    /// Number of input channels (0 for sources)
    fn num_inputs(&self) -> usize { 0 }

    /// Number of output channels
    fn num_outputs(&self) -> usize { 1 }

    /// Native sample rate of this node, if it has one
    /// 
    /// Sources with fixed sample rates (e.g., sample players) return `Some(rate)`.
    /// Effects and sinks that work at any rate return `None`.
    /// 
    /// When adding a node with a native rate different from the graph's rate,
    /// Klingt will automatically create a sub-graph with resampling.
    fn native_sample_rate(&self) -> Option<u32> { None }
}
