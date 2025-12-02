//! Core node trait and context types.

use dasp_graph::{Buffer, Input};

/// Information available during audio processing.
///
/// Passed to every [`AudioNode::process`] call. Contains the graph's sample rate
/// and the buffer size (always 64 samples in the current implementation).
#[derive(Clone, Copy, Debug)]
pub struct ProcessContext {
    /// Sample rate of the graph in Hz (e.g., 44100, 48000)
    pub sample_rate: u32,
    /// Number of samples per buffer (currently always 64)
    pub buffer_size: usize,
}

/// Unique identifier for a node within a graph.
///
/// You typically don't interact with this directly - use [`Handle`](crate::Handle) instead.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(pub(crate) u32);

/// The core trait for audio processing nodes.
///
/// Implement this trait to create custom audio nodes. Nodes can be:
/// - **Sources**: Generate audio (0 inputs, 1+ outputs) - oscillators, sample players
/// - **Effects**: Process audio (1+ inputs, 1+ outputs) - gain, filters, delays
/// - **Sinks**: Consume audio (1+ inputs, 0 outputs) - device outputs, recorders
///
/// # Message-Based Parameters
///
/// Instead of shared mutable state, nodes receive parameter updates via messages.
/// Define your message type and handle it at the start of `process()`:
///
/// ```
/// use klingt::{AudioNode, ProcessContext};
/// use dasp_graph::{Buffer, Input};
///
/// enum MyMessage {
///     SetFrequency(f32),
///     SetVolume(f32),
/// }
///
/// struct MyOscillator {
///     frequency: f32,
///     volume: f32,
///     phase: f32,
/// }
///
/// impl AudioNode for MyOscillator {
///     type Message = MyMessage;
///
///     fn process(
///         &mut self,
///         ctx: &ProcessContext,
///         messages: impl Iterator<Item = MyMessage>,
///         _inputs: &[Input],
///         outputs: &mut [Buffer],
///     ) {
///         // Handle parameter updates first
///         for msg in messages {
///             match msg {
///                 MyMessage::SetFrequency(f) => self.frequency = f,
///                 MyMessage::SetVolume(v) => self.volume = v,
///             }
///         }
///
///         // Generate audio
///         for sample in outputs[0].iter_mut() {
///             *sample = (self.phase * std::f32::consts::TAU).sin() * self.volume;
///             self.phase = (self.phase + self.frequency / ctx.sample_rate as f32) % 1.0;
///         }
///     }
///
///     fn num_outputs(&self) -> usize { 1 }
/// }
/// ```
///
/// # No Messages Needed?
///
/// If your node doesn't need runtime parameter updates, use `()` as the message type:
///
/// ```
/// # use klingt::{AudioNode, ProcessContext};
/// # use dasp_graph::{Buffer, Input};
/// struct FixedTone { /* ... */ }
///
/// impl AudioNode for FixedTone {
///     type Message = (); // No messages
///     
///     fn process(
///         &mut self,
///         ctx: &ProcessContext,
///         _messages: impl Iterator<Item = ()>,
///         _inputs: &[Input],
///         outputs: &mut [Buffer],
///     ) {
///         // Just generate audio, ignore messages
///         # let _ = (ctx, outputs);
///     }
/// }
/// ```
pub trait AudioNode: Send + 'static {
    /// Message type for parameter updates.
    ///
    /// Use a custom enum for nodes with parameters, or `()` for nodes without.
    type Message: Send + 'static;

    /// Process one block of audio.
    ///
    /// Called once per audio block (64 samples). Your implementation should:
    /// 1. Drain and handle all pending messages
    /// 2. Read from `inputs` (if any)
    /// 3. Write to `outputs`
    ///
    /// # Arguments
    ///
    /// - `ctx` - Sample rate and buffer size information
    /// - `messages` - Iterator of pending parameter messages (drain it!)
    /// - `inputs` - Audio inputs from connected nodes
    /// - `outputs` - Audio output buffers to fill
    fn process(
        &mut self,
        ctx: &ProcessContext,
        messages: impl Iterator<Item = Self::Message>,
        inputs: &[Input],
        outputs: &mut [Buffer],
    );

    /// Number of audio input channels (0 for sources).
    fn num_inputs(&self) -> usize { 0 }

    /// Number of audio output channels.
    fn num_outputs(&self) -> usize { 1 }

    /// Native sample rate of this node, if it has one.
    /// 
    /// Sources with fixed sample rates (e.g., sample players with pre-decoded
    /// audio) should return `Some(rate)`. Effects and sinks that work at any
    /// rate should return `None` (the default).
    /// 
    /// When adding a node with a native rate different from the output device,
    /// [`Klingt`](crate::Klingt) will automatically create a sub-graph at the
    /// node's native rate with resampling to match the output.
    fn native_sample_rate(&self) -> Option<u32> { None }
}
