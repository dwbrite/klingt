//! Audio sink nodes (outputs with no audio outputs)

mod rtrb_sink;

#[cfg(feature = "cpal_sink")]
mod cpal_sink;

pub use rtrb_sink::RtrbSink;

#[cfg(feature = "cpal_sink")]
pub use cpal_sink::CpalSink;
