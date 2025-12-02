//! Audio effect nodes (processors with audio inputs and outputs)

mod gain;
mod mixer;
mod slew_limiter;

pub use gain::{Gain, GainMessage};
pub use mixer::Mixer;
pub use slew_limiter::{SlewLimiter, SlewLimiterMessage};
