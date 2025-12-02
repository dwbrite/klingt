//! Klingt v2 - Audio graph with message-passing parameter control
//!
//! Design principles:
//! - Each graph has a fixed sample rate (from device or explicit)
//! - Nodes receive parameters via message ring buffers, not shared state
//! - No Arc, no locks on the audio thread
//! - CPAL devices are discoverable, sinks are just nodes
//! - Automatic sample rate conversion via sub-graphs

mod node;
mod graph;
mod device;
mod klingt;
pub mod nodes;

pub use node::{AudioNode, ProcessContext, NodeId};
pub use graph::{AudioGraph, NodeHandle};
pub use device::CpalDevice;
pub use klingt::{Klingt, Handle};
