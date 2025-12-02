//! Example: Creating a custom audio node
//!
//! This demonstrates how to implement the `AudioNode` trait to create
//! your own audio processing nodes with message-based parameter control.
//!
//! Run with: cargo run --example custom_node --features cpal_sink

use std::thread::sleep;
use std::time::{Duration, Instant};

use dasp_graph::{Buffer, Input};
use klingt::{AudioNode, CpalDevice, Klingt, ProcessContext};
use klingt::nodes::Gain;

// =============================================================================
// Step 1: Define your message type for runtime parameter control
// =============================================================================

/// Messages to control our square wave oscillator
#[derive(Clone, Copy, Debug)]
pub enum SquareMessage {
    /// Set the frequency in Hz
    SetFrequency(f32),
    /// Set the pulse width (0.0 to 1.0, where 0.5 is a standard square wave)
    SetPulseWidth(f32),
    /// Set the amplitude (0.0 to 1.0)
    SetAmplitude(f32),
}

// =============================================================================
// Step 2: Define your node struct with internal state
// =============================================================================

/// A square wave oscillator with variable pulse width
pub struct Square {
    frequency: f32,
    pulse_width: f32,
    amplitude: f32,
    phase: f32,
}

impl Square {
    /// Create a new square wave oscillator at the given frequency
    pub fn new(frequency: f32) -> Self {
        Self {
            frequency,
            pulse_width: 0.5, // Standard square wave
            amplitude: 0.25,  // -12dB, safe default
            phase: 0.0,
        }
    }

    /// Set initial pulse width (builder pattern)
    pub fn with_pulse_width(mut self, pw: f32) -> Self {
        self.pulse_width = pw.clamp(0.0, 1.0);
        self
    }
}

// =============================================================================
// Step 3: Implement AudioNode
// =============================================================================

impl AudioNode for Square {
    type Message = SquareMessage;

    fn process(
        &mut self,
        ctx: &ProcessContext,
        messages: impl Iterator<Item = SquareMessage>,
        _inputs: &[Input],
        outputs: &mut [Buffer],
    ) {
        // 1. Handle any pending messages (parameter updates)
        for msg in messages {
            match msg {
                SquareMessage::SetFrequency(f) => self.frequency = f.max(0.0),
                SquareMessage::SetPulseWidth(pw) => self.pulse_width = pw.clamp(0.0, 1.0),
                SquareMessage::SetAmplitude(a) => self.amplitude = a.clamp(0.0, 1.0),
            }
        }

        // 2. Generate audio samples
        if outputs.is_empty() {
            return;
        }

        let phase_inc = self.frequency / ctx.sample_rate as f32;

        for sample in outputs[0].iter_mut() {
            // Square wave: high when phase < pulse_width, low otherwise
            *sample = if self.phase < self.pulse_width {
                self.amplitude
            } else {
                -self.amplitude
            };

            // Advance and wrap phase
            self.phase += phase_inc;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }
    }

    // This is a source node: no inputs, one output
    fn num_inputs(&self) -> usize { 0 }
    fn num_outputs(&self) -> usize { 1 }
}

// =============================================================================
// Main: Use the custom node just like built-in nodes
// =============================================================================

fn main() {
    // Set up audio output
    let device = CpalDevice::default_output().expect("No audio device found");
    println!("Using: {} @ {} Hz", device.name(), device.sample_rate());

    let mut klingt = Klingt::new(device.sample_rate())
        .with_output(device.create_sink());

    // Create our custom square wave oscillator
    let mut square = klingt.add(Square::new(220.0).with_pulse_width(0.5));
    
    // Add some gain to control volume
    let gain = klingt.add(Gain::new(0.5));
    
    klingt.connect(&square, &gain);
    klingt.output(&gain);

    println!("Playing square wave with PWM modulation... Ctrl+C to stop\n");

    // Audio processing loop with pulse width modulation
    let start = Instant::now();
    let rate = klingt.sample_rate() as f64;
    let mut blocks = 0u64;

    loop {
        let elapsed = start.elapsed().as_secs_f32();
        
        // Modulate pulse width with a slow sine wave (0.25 to 0.75)
        let pw = 0.5 + 0.25 * (elapsed * 0.5).sin();
        square.send(SquareMessage::SetPulseWidth(pw)).ok();
        
        // Also slowly modulate frequency for a siren effect
        let freq = 220.0 + 110.0 * (elapsed * 0.2).sin();
        square.send(SquareMessage::SetFrequency(freq)).ok();

        // Process audio blocks to stay ahead of playback
        let target = (start.elapsed().as_secs_f64() * rate / 64.0) as u64 + 4;
        while blocks < target {
            klingt.process();
            blocks += 1;
        }

        sleep(Duration::from_millis(10));
    }
}
