//! Simple audio file player
//!
//! Run with: cargo run --example audio_player --features cpal_sink

use std::thread::sleep;
use std::time::{Duration, Instant};

use symphonium::SymphoniumLoader;

use klingt::Klingt;
use klingt::nodes::SamplePlayer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut klingt = Klingt::default_output().ok_or("No audio device")?;

    // create file player
    let decoded = SymphoniumLoader::new().load_f32("lowtide.ogg", None)?;
    let mut player = SamplePlayer::new(
        decoded.as_interleaved(),
        decoded.channels(),
        decoded.sample_rate,
    );
    player.set_looping(true);
    
    // add player and set primary output
    let player_handle = klingt.add(player);
    klingt.output(&player_handle);

    


    // and main loop type shit
    println!("Playing... Ctrl+C to stop");
    
    let start = Instant::now();
    let rate = klingt.sample_rate() as f64;
    let mut blocks = 0u64;

    loop {
        let target = (start.elapsed().as_secs_f64() * rate / 64.0) as u64 + 6; // 6 blocks buffer
        while blocks < target {
            klingt.process();
            blocks += 1;
        }
        sleep(Duration::from_micros(500));
    }
}
