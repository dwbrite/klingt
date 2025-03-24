
#[cfg(all(feature = "cpal_sink", feature = "std"))]
mod cpalmonosink;

#[cfg(all(feature = "cpal_sink", feature = "std"))]
mod cpalstereosink;

#[cfg(all(feature = "cpal_sink", feature = "std"))]
pub use cpalmonosink::*;

#[cfg(all(feature = "cpal_sink", feature = "std"))]
pub use cpalstereosink::*;

mod rtrbsink;
pub use rtrbsink::*;