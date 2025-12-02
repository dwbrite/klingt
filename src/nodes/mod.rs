//! Built-in audio nodes.
//!
//! Nodes are organized into three categories:
//!
//! ## Sources ([`source`])
//!
//! Generate audio with no audio inputs:
//! - [`Sine`] - Sine wave oscillator with frequency/amplitude control
//! - [`SamplePlayer`] - Play pre-decoded audio samples
//! - [`ResamplingSource`] - Read from ring buffer with sample rate conversion (internal use)
//!
//! ## Effects ([`effect`])
//!
//! Process audio (inputs → outputs):
//! - [`Gain`] - Volume control with smoothing
//! - [`Mixer`] - Sum multiple inputs together
//! - [`SlewLimiter`] - Smooth rapid changes (for control signals)
//!
//! ## Sinks ([`sink`])
//!
//! Consume audio with no audio outputs:
//! - [`CpalSink`] - Output to system audio device (requires `cpal_sink` feature)
//! - [`RtrbSink`] - Write to ring buffer (internal use for sub-graphs)
//!
//! # Message Types
//!
//! Most nodes have associated message types for runtime parameter control:
//! - [`SineMessage`] - Control [`Sine`] frequency and amplitude
//! - [`PlayerMessage`] - Control [`SamplePlayer`] playback (play/pause/seek)
//! - [`GainMessage`] - Control [`Gain`] level
//! - [`SlewLimiterMessage`] - Control [`SlewLimiter`] rate
//!
//! Nodes without parameters (like [`Mixer`]) use `()` as their message type.

pub mod source;
pub mod effect;
pub mod sink;

// Re-export common types at the top level for convenience
pub use source::{Sine, SineMessage, SamplePlayer, PlayerMessage, ResamplingSource, ResamplingSourceMessage};
pub use effect::{Gain, GainMessage, Mixer, SlewLimiter, SlewLimiterMessage};
pub use sink::RtrbSink;

#[cfg(feature = "cpal_sink")]
pub use sink::CpalSink;
