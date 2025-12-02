//! Audio source nodes - generators with no audio inputs.
//!
//! Sources generate audio from nothing (oscillators) or from data (sample players).
//! They have 0 audio inputs and 1+ audio outputs.
//!
//! # Available Sources
//!
//! - [`Sine`] - Sine wave oscillator
//! - [`SamplePlayer`] - Play pre-decoded audio samples
//! - [`ResamplingSource`] - Internal node for sample rate conversion

mod sine;
mod player;
mod resampling_source;

pub use sine::{Sine, SineMessage};
pub use player::{SamplePlayer, PlayerMessage};
pub use resampling_source::{ResamplingSource, ResamplingSourceMessage};
