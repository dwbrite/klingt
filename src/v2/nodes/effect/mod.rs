//! Audio effect nodes (processors with audio inputs and outputs)

mod gain;
mod slew_limiter;

pub use gain::{Gain, GainMessage};
pub use slew_limiter::{SlewLimiter, SlewLimiterMessage};
