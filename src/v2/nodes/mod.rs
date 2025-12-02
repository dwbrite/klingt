//! Built-in audio nodes
//! 
//! Nodes are organized into three categories:
//! - `source`: Generate audio (no audio inputs) - oscillators, sample players
//! - `effect`: Process audio (inputs → outputs) - gain, filters, delays
//! - `sink`: Consume audio (no audio outputs) - device outputs, recorders

pub mod source;
pub mod effect;
pub mod sink;

// Re-export common types at the top level for convenience
pub use source::{Sine, SineMessage, SamplePlayer, PlayerMessage, ResamplingSource, ResamplingSourceMessage};
pub use effect::{Gain, GainMessage, SlewLimiter, SlewLimiterMessage};
pub use sink::RtrbSink;

#[cfg(feature = "cpal_sink")]
pub use sink::CpalSink;
