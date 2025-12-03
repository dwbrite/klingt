//! Integration tests for klingt
//!
//! These tests require audio output and are meant to be run manually
//! to verify audio functionality.

use std::thread::sleep;
use std::time::{Duration, Instant};

use klingt::nodes::effect::{Gain, Mixer, SlewLimiter};
use klingt::nodes::source::Sine;
use klingt::Klingt;

#[cfg(feature = "cpal_sink")]
use klingt::CpalDevice;

/// Helper to run the audio loop for a given duration
fn run_for(klingt: &mut Klingt, seconds: f32) {
    let start = Instant::now();
    let duration = Duration::from_secs_f32(seconds);
    let rate = klingt.sample_rate() as f64;
    let mut blocks = 0u64;

    while start.elapsed() < duration {
        let target = (start.elapsed().as_secs_f64() * rate / 64.0) as u64 + 4;
        while blocks < target {
            klingt.process();
            blocks += 1;
        }
        sleep(Duration::from_micros(500));
    }
}

#[test]
#[ignore] // Requires audio hardware
#[cfg(feature = "cpal_sink")]
fn sine_wave_5s() {
    let mut klingt = Klingt::default_output().expect("No audio device");
    
    let sine = klingt.add(Sine::new(440.0));
    klingt.output(&sine);
    
    println!("Playing 440Hz sine wave for 5 seconds...");
    let start = Instant::now();
    run_for(&mut klingt, 5.0);
    println!("Done in {:?}", start.elapsed());
}

#[test]
#[ignore] // Requires audio hardware
#[cfg(feature = "cpal_sink")]
fn chord_with_mixer() {
    let mut klingt = Klingt::default_output().expect("No audio device");
    
    // C major chord: C4, E4, G4
    let c = klingt.add(Sine::new(261.63));
    let e = klingt.add(Sine::new(329.63));
    let g = klingt.add(Sine::new(392.00));
    
    let mixer = klingt.add(Mixer::stereo());
    klingt.connect(&c, &mixer);
    klingt.connect(&e, &mixer);
    klingt.connect(&g, &mixer);
    
    // Attenuate to prevent clipping (3 sines summed)
    let gain = klingt.add(Gain::new(0.33));
    klingt.connect(&mixer, &gain);
    klingt.output(&gain);
    
    println!("Playing C major chord for 3 seconds...");
    let start = Instant::now();
    run_for(&mut klingt, 3.0);
    println!("Done in {:?}", start.elapsed());
}

#[test]
#[ignore] // Requires audio hardware  
#[cfg(feature = "cpal_sink")]
fn slew_limiter_smoothing() {
    let mut klingt = Klingt::default_output().expect("No audio device");
    
    // Use a low frequency square-ish wave approximation to test slew limiting
    let sine = klingt.add(Sine::new(100.0).with_amplitude(0.5));
    let slew = klingt.add(SlewLimiter::new(0.001)); // Very aggressive slewing
    let gain = klingt.add(Gain::new(0.5));
    
    klingt.connect(&sine, &slew);
    klingt.connect(&slew, &gain);
    klingt.output(&gain);
    
    println!("Playing slew-limited sine for 3 seconds...");
    let start = Instant::now();
    run_for(&mut klingt, 3.0);
    println!("Done in {:?}", start.elapsed());
}

#[test]
#[ignore] // Requires audio hardware
#[cfg(feature = "cpal_sink")]
fn runtime_frequency_change() {
    use klingt::nodes::SineMessage;
    
    let mut klingt = Klingt::default_output().expect("No audio device");
    
    let mut sine = klingt.add(Sine::new(220.0));
    klingt.output(&sine);
    
    println!("Playing ascending tones...");
    let start = Instant::now();
    
    // Play ascending scale
    let frequencies = [220.0, 246.94, 261.63, 293.66, 329.63, 349.23, 392.00, 440.0];
    for freq in frequencies {
        sine.send(SineMessage::SetFrequency(freq)).ok();
        run_for(&mut klingt, 0.5);
    }
    
    println!("Done in {:?}", start.elapsed());
}

#[test]
#[ignore] // Requires audio hardware
#[cfg(feature = "cpal_sink")]
fn device_selection() {
    let devices = CpalDevice::list_outputs();
    assert!(!devices.is_empty(), "No audio devices found");
    
    println!("Found {} audio devices:", devices.len());
    for (i, device) in devices.iter().enumerate() {
        println!("  [{}] {} ({} Hz, {} ch)", 
            i, device.name(), device.sample_rate(), device.channels());
    }
    
    // Use first device
    let device = &devices[0];
    let mut klingt = Klingt::new(device.sample_rate())
        .with_output(device.create_sink());
    
    let sine = klingt.add(Sine::new(440.0));
    klingt.output(&sine);
    
    println!("Playing on '{}' for 2 seconds...", device.name());
    run_for(&mut klingt, 2.0);
    println!("Done");
}

// Unit tests that don't require audio hardware

#[test]
fn klingt_sample_rate() {
    let klingt = Klingt::new(48000);
    assert_eq!(klingt.sample_rate(), 48000);
    
    let klingt = Klingt::new(44100);
    assert_eq!(klingt.sample_rate(), 44100);
}

#[test]
fn node_creation() {
    let sine = Sine::new(440.0);
    assert_eq!(sine.frequency(), 440.0);
    assert_eq!(sine.amplitude(), 0.25); // default amplitude
    
    let sine2 = Sine::new(880.0).with_amplitude(0.5);
    assert_eq!(sine2.frequency(), 880.0);
    assert_eq!(sine2.amplitude(), 0.5);
}

#[test]
fn gain_creation() {
    let gain = Gain::new(0.5);
    assert_eq!(gain.gain(), 0.5);
    
    let gain2 = Gain::new(1.5);
    assert_eq!(gain2.gain(), 1.5);
}
