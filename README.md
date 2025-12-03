# Klingt

A lock-free audio graph library with message-passing parameter control.

## Features

- **Lock-free audio thread** – No allocations, no `Arc`/`Mutex` on the hot path
- **Automatic resampling** – Nodes at different sample rates just work
- **Simple API** – Add nodes, connect them, call `process()`

## Quick Start

```rust
use klingt::{Klingt, nodes::Sine};

fn main() {
    // Create engine with default audio device
    let mut klingt = Klingt::default_output().expect("No audio device");

    // Add a sine oscillator and connect to output
    let sine = klingt.add(Sine::new(440.0));
    klingt.output(&sine);

    // Main audio loop
    loop {
        klingt.process();
        std::thread::sleep(std::time::Duration::from_micros(500));
    }
}
```

## Runtime Parameter Control

Send messages to nodes without locks:

```rust
use klingt::{Klingt, nodes::{Sine, SineMessage, Gain}};

let mut klingt = Klingt::default_output().unwrap();

let mut sine = klingt.add(Sine::new(440.0));
let gain = klingt.add(Gain::new(0.5));

klingt.connect(&sine, &gain);
klingt.output(&gain);

// Change frequency at runtime (lock-free!)
sine.send(SineMessage::SetFrequency(880.0)).ok();
```

## Automatic Sample Rate Conversion

Add nodes at their native sample rate – Klingt handles the rest:

```rust
// Audio file at 48kHz + device at 44.1kHz = automatic resampling
let player = SamplePlayer::new(samples, 2, 48000);
let handle = klingt.add(player);  // Sub-graph created automatically
klingt.output(&handle);           // Routed through resampler
```

## Built-in Nodes

- **Sources**: `Sine`, `SamplePlayer`
- **Effects**: `Gain`, `Mixer`, `SlewLimiter`
- **Sinks**: `CpalSink` (with `cpal_sink` feature)

## Custom Nodes

Implement the `AudioNode` trait to create your own:

```rust
use klingt::{AudioNode, ProcessContext};
use dasp_graph::{Buffer, Input};

pub enum SquareMessage {
    SetFrequency(f32),
    SetPulseWidth(f32),
}

pub struct Square {
    frequency: f32,
    pulse_width: f32,
    phase: f32,
}

impl AudioNode for Square {
    type Message = SquareMessage;

    fn process(
        &mut self,
        ctx: &ProcessContext,
        messages: impl Iterator<Item = SquareMessage>,
        _inputs: &[Input],
        outputs: &mut [Buffer],
    ) {
        // Handle messages
        for msg in messages {
            match msg {
                SquareMessage::SetFrequency(f) => self.frequency = f,
                SquareMessage::SetPulseWidth(pw) => self.pulse_width = pw,
            }
        }

        // Generate audio
        let phase_inc = self.frequency / ctx.sample_rate as f32;
        for sample in outputs[0].iter_mut() {
            *sample = if self.phase < self.pulse_width { 0.25 } else { -0.25 };
            self.phase = (self.phase + phase_inc) % 1.0;
        }
    }

    fn num_outputs(&self) -> usize { 1 }
}
```

## Feature Flags

- `cpal_sink` – Enable CPAL audio output
- `std` – Enable standard library (enabled by default)

## License

MIT
