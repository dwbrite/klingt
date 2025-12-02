//! Simple example: play a sine wave with configurable output device
//!
//! Run with: cargo run --example simple_sine --features cpal_sink
//!
//! Lists available devices and lets you pick one, then plays a 440Hz sine.

use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

use klingt::v2::{CpalDevice, Klingt};
use klingt::v2::nodes::{Sine, SineMessage};

fn main() {
    // List available output devices
    let devices = CpalDevice::list_outputs();
    
    if devices.is_empty() {
        eprintln!("No audio output devices found!");
        return;
    }

    println!("Available audio output devices:");
    for (i, device) in devices.iter().enumerate() {
        println!(
            "  [{}] {} ({}Hz, {} ch)",
            i,
            device.name(),
            device.sample_rate(),
            device.channels()
        );
    }

    // Let user pick a device (or default to 0)
    print!("\nSelect device [0]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let choice: usize = input.trim().parse().unwrap_or(0);

    let device = devices.into_iter().nth(choice).unwrap_or_else(|| {
        println!("Invalid choice, using default device");
        CpalDevice::default_output().expect("No default device")
    });

    println!(
        "\nUsing: {} @ {}Hz",
        device.name(),
        device.sample_rate()
    );

    // Create Klingt with the selected device as output
    let mut klingt = Klingt::new(device.sample_rate())
        .with_output(device.create_sink());

    // Add a sine wave
    let mut sine_handle = klingt.add(Sine::new(440.0).with_amplitude(0.25));

    // Connect to output
    klingt.output(&sine_handle);

    println!("Playing 440Hz sine wave...");
    println!("Press Ctrl+C to stop\n");

    let sample_rate = klingt.sample_rate();
    
    // Pre-fill buffer
    for _ in 0..8 {
        klingt.process();
    }
    
    let mut samples_generated: u64 = 8 * 64 * 2;
    let audio_start = std::time::Instant::now();

    loop {
        let elapsed = audio_start.elapsed().as_secs_f64();
        let samples_should_have_played = (elapsed * sample_rate as f64 * 2.0) as u64;
        
        let buffer_ahead = 880u64;
        
        if samples_generated < samples_should_have_played + buffer_ahead {
            klingt.process();
            samples_generated += 64 * 2;
        } else {
            sleep(Duration::from_micros(500));
        }

        // Modulate frequency slowly
        let freq = 440.0 + 220.0 * (elapsed as f32 * 0.5 * std::f32::consts::PI).sin();
        let _ = sine_handle.send(SineMessage::SetFrequency(freq));
    }
}
