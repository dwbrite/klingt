//! CPAL device discovery and sink creation

use alloc::string::String;
use alloc::vec::Vec;

#[cfg(feature = "cpal_sink")]
use cpal::traits::{DeviceTrait, HostTrait};

/// A discovered audio output device
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
    /// Get the default output device
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
    
    /// List all available output devices
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
    
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    
    pub fn channels(&self) -> u16 {
        self.channels
    }
    
    /// Create a sink node that outputs to this device
    #[cfg(feature = "cpal_sink")]
    pub fn create_sink(&self) -> crate::v2::nodes::CpalSink {
        crate::v2::nodes::CpalSink::new(&self.device, &self.config)
    }
}
