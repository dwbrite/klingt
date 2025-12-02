//! # Klingt
//!
//! A lock-free audio graph library with message-passing parameter control.
//!
//! ## Quick Start
//!
//! The simplest way to play audio is with [`Klingt::default_output`]:
//!
//! ```no_run
//! use klingt::{Klingt, nodes::Sine};
//!
//! // Create engine with default audio device
//! let mut klingt = Klingt::default_output().expect("No audio device");
//!
//! // Add a sine oscillator and connect to output
//! let sine = klingt.add(Sine::new(440.0));
//! klingt.output(&sine);
//!
//! // Main audio loop - call process() repeatedly
//! loop {
//!     klingt.process();
//!     std::thread::sleep(std::time::Duration::from_micros(500));
//! }
//! ```
//!
//! ## Core Concepts
//!
//! ### Nodes and Handles
//!
//! Audio processing is done by **nodes** that implement [`AudioNode`]. When you add a node
//! to Klingt, you get back a [`Handle`] that lets you:
//! - Connect nodes together with [`Klingt::connect`]
//! - Send parameter updates with [`Handle::send`]
//!
//! ```no_run
//! # use klingt::{Klingt, nodes::{Sine, SineMessage, Gain}};
//! # let mut klingt = Klingt::default_output().unwrap();
//! let mut sine = klingt.add(Sine::new(440.0));
//! let gain = klingt.add(Gain::new(0.5));
//!
//! klingt.connect(&sine, &gain);
//! klingt.output(&gain);
//!
//! // Change frequency at runtime (lock-free!)
//! sine.send(SineMessage::SetFrequency(880.0)).ok();
//! ```
//!
//! ### Automatic Sample Rate Conversion
//!
//! Klingt automatically handles sample rate mismatches. If you add a node that
//! has a different native sample rate (like a pre-decoded audio file), Klingt
//! creates a sub-graph at that rate and resamples to match the output device:
//!
//! ```ignore
//! // Audio file at 48000Hz + device at 44100Hz = automatic resampling
//! let player = SamplePlayer::new(samples, 2, 48000);
//! let handle = klingt.add(player); // Sub-graph created automatically
//! klingt.output(&handle);          // Routed through resampler
//! ```
//!
//! ### Message Passing (No Locks!)
//!
//! All parameter updates use lock-free ring buffers. The audio thread never
//! blocks waiting for the main thread. Messages are processed at the start of
//! each audio block (64 samples by default).
//!
//! ## Built-in Nodes
//!
//! See the [`nodes`] module for available nodes:
//!
//! - **Sources**: [`Sine`](nodes::Sine), [`SamplePlayer`](nodes::SamplePlayer)
//! - **Effects**: [`Gain`](nodes::Gain), [`Mixer`](nodes::Mixer), [`SlewLimiter`](nodes::SlewLimiter)
//! - **Sinks**: [`CpalSink`](nodes::CpalSink) (with `cpal_sink` feature)
//!
//! ## Custom Nodes
//!
//! Implement [`AudioNode`] to create your own nodes. Here's a complete example
//! of a square wave oscillator with message-based parameter control:
//!
//! ```
//! use klingt::{AudioNode, ProcessContext};
//! use dasp_graph::{Buffer, Input};
//!
//! // Define messages for runtime parameter control
//! #[derive(Clone, Copy, Debug)]
//! pub enum SquareMessage {
//!     SetFrequency(f32),
//!     SetPulseWidth(f32),  // 0.0 to 1.0, where 0.5 is a standard square
//!     SetAmplitude(f32),
//! }
//!
//! pub struct Square {
//!     frequency: f32,
//!     pulse_width: f32,
//!     amplitude: f32,
//!     phase: f32,
//! }
//!
//! impl Square {
//!     pub fn new(frequency: f32) -> Self {
//!         Self {
//!             frequency,
//!             pulse_width: 0.5,
//!             amplitude: 0.25,
//!             phase: 0.0,
//!         }
//!     }
//! }
//!
//! impl AudioNode for Square {
//!     type Message = SquareMessage;
//!
//!     fn process(
//!         &mut self,
//!         ctx: &ProcessContext,
//!         messages: impl Iterator<Item = SquareMessage>,
//!         _inputs: &[Input],
//!         outputs: &mut [Buffer],
//!     ) {
//!         // 1. Handle messages first (parameter updates)
//!         for msg in messages {
//!             match msg {
//!                 SquareMessage::SetFrequency(f) => self.frequency = f.max(0.0),
//!                 SquareMessage::SetPulseWidth(pw) => self.pulse_width = pw.clamp(0.0, 1.0),
//!                 SquareMessage::SetAmplitude(a) => self.amplitude = a.clamp(0.0, 1.0),
//!             }
//!         }
//!
//!         // 2. Generate audio
//!         let phase_inc = self.frequency / ctx.sample_rate as f32;
//!         
//!         for sample in outputs[0].iter_mut() {
//!             // Square wave: high when phase < pulse_width, low otherwise
//!             *sample = if self.phase < self.pulse_width {
//!                 self.amplitude
//!             } else {
//!                 -self.amplitude
//!             };
//!
//!             // Advance and wrap phase
//!             self.phase += phase_inc;
//!             if self.phase >= 1.0 {
//!                 self.phase -= 1.0;
//!             }
//!         }
//!     }
//!
//!     fn num_outputs(&self) -> usize { 1 }
//! }
//! ```
//!
//! Then use it like any built-in node:
//!
//! ```ignore
//! let mut square = klingt.add(Square::new(440.0));
//! klingt.output(&square);
//!
//! // Modulate pulse width for PWM synthesis
//! square.send(SquareMessage::SetPulseWidth(0.25)).ok();
//! ```
//!
//! ### Node Types
//!
//! The three types of nodes differ by their input/output counts:
//!
//! | Type   | Inputs | Outputs | Examples |
//! |--------|--------|---------|----------|
//! | Source | 0      | 1+      | Oscillators, sample players |
//! | Effect | 1+     | 1+      | Gain, filters, delays |
//! | Sink   | 1+     | 0       | Audio output, recorders |
//!
//! Override [`num_inputs`](AudioNode::num_inputs) and [`num_outputs`](AudioNode::num_outputs)
//! to define your node's channel configuration.
//!
//! ## Feature Flags
//!
//! - `cpal_sink` - Enable CPAL audio output (adds [`CpalDevice`] and [`CpalSink`](nodes::CpalSink))
//! - `std` - Enable standard library (enabled by default)
//!
//! ## Design Principles
//!
//! - **Lock-free audio thread**: No `Arc`, no `Mutex` on the hot path
//! - **Message passing**: Parameters sent via ring buffers, not shared state
//! - **Automatic resampling**: Nodes at different sample rates just work
//! - **Fixed block size**: 64 samples per block (from dasp_graph)

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod node;
mod graph;
mod klingt;
pub mod nodes;

#[cfg(feature = "cpal_sink")]
mod device;

pub use node::{AudioNode, ProcessContext, NodeId};
pub use klingt::{Klingt, Handle};

#[cfg(feature = "cpal_sink")]
pub use device::CpalDevice;
