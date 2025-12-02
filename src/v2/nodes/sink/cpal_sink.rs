//! CPAL audio output sink

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, SupportedStreamConfig};
use dasp_graph::{Buffer, Input};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use crate::v2::node::{AudioNode, ProcessContext};

/// A sink that outputs audio to a CPAL device
///
/// The CPAL stream runs on its own thread; this node feeds samples
/// into a ring buffer that the stream consumes.
pub struct CpalSink {
    buffer: Producer<f32>,
    channels: usize,
    /// Tracks how many samples CPAL has consumed
    samples_consumed: Arc<AtomicUsize>,
    /// Tracks underrun state for diagnostics
    had_underrun: Arc<AtomicBool>,
}

impl CpalSink {
    /// Create a new sink for the given device and config
    pub fn new(device: &cpal::Device, config: &SupportedStreamConfig) -> Self {
        let channels = config.channels() as usize;
        let sample_format = config.sample_format();
        let stream_config = config.config();
        let sample_rate = stream_config.sample_rate.0;

        // Ring buffer sized for ~100ms of audio to handle scheduling jitter
        let buffer_samples = ((sample_rate as f32 * 0.1) as usize) * channels;
        let buffer_size = buffer_samples.next_power_of_two().max(8192);
        let (producer, consumer) = RingBuffer::<f32>::new(buffer_size);

        let samples_consumed = Arc::new(AtomicUsize::new(0));
        let samples_consumed_clone = samples_consumed.clone();

        let had_underrun = Arc::new(AtomicBool::new(false));
        let had_underrun_clone = had_underrun.clone();

        // Spawn stream on dedicated thread
        let device = device.clone();
        std::thread::spawn(move || {
            let stream = build_stream(
                &device,
                sample_format,
                &stream_config,
                consumer,
                samples_consumed_clone,
                had_underrun_clone,
            )
            .expect("Failed to build output stream");

            stream.play().expect("Failed to start audio stream");

            // Keep thread alive - stream lives as long as this thread
            loop {
                std::thread::park();
            }
        });

        Self {
            buffer: producer,
            channels,
            samples_consumed,
            had_underrun,
        }
    }

    /// Returns how many samples have been played
    #[inline]
    pub fn samples_consumed(&self) -> usize {
        self.samples_consumed.load(Ordering::Relaxed)
    }

    /// Returns available space in the buffer (in samples)
    #[inline]
    pub fn buffer_available(&self) -> usize {
        self.buffer.slots()
    }

    /// Check and clear the underrun flag
    pub fn check_underrun(&self) -> bool {
        self.had_underrun.swap(false, Ordering::Relaxed)
    }
}

fn build_stream(
    device: &cpal::Device,
    sample_format: SampleFormat,
    stream_config: &cpal::StreamConfig,
    mut consumer: Consumer<f32>,
    samples_consumed: Arc<AtomicUsize>,
    had_underrun: Arc<AtomicBool>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    match sample_format {
        SampleFormat::F32 => device.build_output_stream(
            stream_config,
            move |data: &mut [f32], _| {
                let mut underrun = false;
                for sample in data.iter_mut() {
                    *sample = consumer.pop().unwrap_or_else(|_| {
                        underrun = true;
                        0.0
                    });
                }
                if underrun {
                    had_underrun.store(true, Ordering::Relaxed);
                }
                samples_consumed.fetch_add(data.len(), Ordering::Relaxed);
            },
            |err| eprintln!("CPAL stream error: {:?}", err),
            None,
        ),
        SampleFormat::I16 => device.build_output_stream(
            stream_config,
            move |data: &mut [i16], _| {
                let mut underrun = false;
                for sample in data.iter_mut() {
                    let s = consumer.pop().unwrap_or_else(|_| {
                        underrun = true;
                        0.0
                    });
                    *sample = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                }
                if underrun {
                    had_underrun.store(true, Ordering::Relaxed);
                }
                samples_consumed.fetch_add(data.len(), Ordering::Relaxed);
            },
            |err| eprintln!("CPAL stream error: {:?}", err),
            None,
        ),
        SampleFormat::U16 => device.build_output_stream(
            stream_config,
            move |data: &mut [u16], _| {
                let mut underrun = false;
                for sample in data.iter_mut() {
                    let s = consumer.pop().unwrap_or_else(|_| {
                        underrun = true;
                        0.0
                    });
                    *sample = ((s.clamp(-1.0, 1.0) + 1.0) * 0.5 * u16::MAX as f32) as u16;
                }
                if underrun {
                    had_underrun.store(true, Ordering::Relaxed);
                }
                samples_consumed.fetch_add(data.len(), Ordering::Relaxed);
            },
            |err| eprintln!("CPAL stream error: {:?}", err),
            None,
        ),
        _ => panic!("Unsupported sample format: {:?}", sample_format),
    }
}

impl AudioNode for CpalSink {
    type Message = (); // No control messages

    fn process(
        &mut self,
        _ctx: &ProcessContext,
        _messages: impl Iterator<Item = ()>,
        inputs: &[Input],
        _outputs: &mut [Buffer],
    ) {
        if inputs.is_empty() {
            return;
        }

        let input = &inputs[0];
        let buffers = input.buffers();

        if buffers.is_empty() {
            return;
        }

        let buffer_len = buffers[0].len();
        let samples_needed = buffer_len * self.channels;

        // Check for overrun (generating faster than consuming)
        if self.buffer.slots() < samples_needed {
            // Skip this block rather than partially write
            return;
        }

        // Interleave channels into ring buffer
        for i in 0..buffer_len {
            for ch in 0..self.channels {
                // Map output channel to source (duplicate mono to stereo if needed)
                let src_ch = ch.min(buffers.len() - 1);
                // Safety: we verified slots above
                let _ = self.buffer.push(buffers[src_ch][i]);
            }
        }
    }

    #[inline]
    fn num_inputs(&self) -> usize { 1 }

    #[inline]
    fn num_outputs(&self) -> usize { 0 }
}
