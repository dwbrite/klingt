//! CPAL device discovery and sink creation.
//!
//! This module provides [`CpalDevice`] for discovering and selecting audio output devices.
//!
//! # Example: List and Select a Device
//!
//! ```no_run
//! use klingt::{Klingt, CpalDevice};
//!
//! // List all available output devices
//! let devices = CpalDevice::list_outputs();
//! for (i, device) in devices.iter().enumerate() {
//!     println!("[{}] {} ({} Hz, {} ch)",
//!         i, device.name(), device.sample_rate(), device.channels());
//! }
//!
//! // Use a specific device
//! let device = &devices[0];
//! let mut klingt = Klingt::new(device.sample_rate())
//!     .with_output(device.create_sink());
//! ```

use alloc::string::String;
use alloc::vec::Vec;

#[cfg(feature = "cpal_sink")]
use cpal::traits::{DeviceTrait, HostTrait};

/// A discovered audio output device.
///
/// Use [`CpalDevice::default_output`] to get the system default, or
/// [`CpalDevice::list_outputs`] to enumerate all available devices.
///
/// Once you have a device, use [`create_sink`](Self::create_sink) to create
/// a [`CpalSink`](crate::nodes::CpalSink) node for audio output.
pub struct CpalDevice {
    #[cfg(feature = "cpal_sink")]
    device: cpal::Device,
    #[cfg(feature = "cpal_sink")]
    config: cpal::SupportedStreamConfig,
    
    name: String,
    sample_rate: u32,
    channels: u16,
}

impl CpalDevice {
    /// Get the system's default output device.
    ///
    /// Returns `None` if no audio device is available.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use klingt::CpalDevice;
    /// if let Some(device) = CpalDevice::default_output() {
    ///     println!("Default: {} at {} Hz", device.name(), device.sample_rate());
    /// }
    /// ```
    #[cfg(feature = "cpal_sink")]
    pub fn default_output() -> Option<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        let name = device.name().unwrap_or_else(|_| "Unknown".into());
        
        Some(Self {
            sample_rate: config.sample_rate().0,
            channels: config.channels(),
            name,
            device,
            config,
        })
    }
    
    #[cfg(not(feature = "cpal_sink"))]
    pub fn default_output() -> Option<Self> {
        None
    }
    
    /// List all available audio output devices.
    ///
    /// Returns an empty list if no devices are found or if enumeration fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use klingt::CpalDevice;
    /// for device in CpalDevice::list_outputs() {
    ///     println!("{}: {} Hz", device.name(), device.sample_rate());
    /// }
    /// ```
    #[cfg(feature = "cpal_sink")]
    pub fn list_outputs() -> Vec<Self> {
        let host = cpal::default_host();
        host.output_devices()
            .map(|devices| {
                devices.filter_map(|device| {
                    let config = device.default_output_config().ok()?;
                    let name = device.name().unwrap_or_else(|_| "Unknown".into());
                    Some(Self {
                        sample_rate: config.sample_rate().0,
                        channels: config.channels(),
                        name,
                        device,
                        config,
                    })
                }).collect()
            })
            .unwrap_or_default()
    }
    
    #[cfg(not(feature = "cpal_sink"))]
    pub fn list_outputs() -> Vec<Self> {
        Vec::new()
    }
    
    /// Get the device name.
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get the device's sample rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    
    /// Get the number of output channels.
    pub fn channels(&self) -> u16 {
        self.channels
    }
    
    /// Create a sink node that outputs audio to this device.
    ///
    /// The returned [`CpalSink`](crate::nodes::CpalSink) should be added to
    /// your graph via [`Klingt::with_output`](crate::Klingt::with_output).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use klingt::{Klingt, CpalDevice};
    /// let device = CpalDevice::default_output().unwrap();
    /// let klingt = Klingt::new(device.sample_rate())
    ///     .with_output(device.create_sink());
    /// ```
    #[cfg(feature = "cpal_sink")]
    pub fn create_sink(&self) -> crate::nodes::CpalSink {
        crate::nodes::CpalSink::new(&self.device, &self.config)
    }
}
