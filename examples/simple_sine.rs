//! Simple sine wave example with device selection
//!
//! Run with: cargo run --example simple_sine --features cpal_sink

use std::thread::sleep;
use std::time::{Duration, Instant};

use klingt::{CpalDevice, Klingt};
use klingt::nodes::{Sine, Mixer, Gain};

fn main() {
    // List available devices and pick the first one
    let devices = CpalDevice::list_outputs();
    for (i, d) in devices.iter().enumerate() {
        println!("[{}] {} ({}Hz)", i, d.name(), d.sample_rate());
    }
    
    let device = devices.into_iter().next().expect("No audio device");
    println!("Using: {}\n", device.name());


    // Build Klingt with selected device
    let mut klingt = Klingt::new(device.sample_rate())
        .with_output(device.create_sink());

    // A major chord: A3 + C#4 + E4
    let root = klingt.add(Sine::new(220.0));    // A3
    let third = klingt.add(Sine::new(277.18));  // C#4
    let fifth = klingt.add(Sine::new(329.63));  // E4
    
    let mixer = klingt.add(Mixer::stereo());
    klingt.connect(&root, &mixer);
    klingt.connect(&third, &mixer);
    klingt.connect(&fifth, &mixer);
    
    // Attenuate to prevent clipping
    let gain = klingt.add(Gain::new(0.33));
    klingt.connect(&mixer, &gain);
    klingt.output(&gain);

    println!("Playing A major chord... Ctrl+C to stop");

    let start = Instant::now();
    let rate = klingt.sample_rate() as f64;
    let mut blocks = 0u64;

    loop {
        let target = (start.elapsed().as_secs_f64() * rate / 64.0) as u64 + 6;
        while blocks < target {
            klingt.process();
            blocks += 1;
        }
        sleep(Duration::from_micros(500));
    }
}
