use cpal::SampleRate;
use dasp_graph::node::Sum;
use dasp_graph::NodeData;
use klingt::nodes::effect::SlewLimiter;
use klingt::nodes::sink::{CpalMonoSink, CpalStereoSink};
use klingt::nodes::source::{BufferedOgg, Sine, Square};
use klingt::NodeVariants;
use klingt::NodeVariants::*;
use petgraph::prelude::NodeIndex;
use std::ops::Index;
use std::thread::sleep;
use std::time::{Duration, Instant};

type Graph = petgraph::graph::Graph<NodeData<NodeVariants>, ()>;
type Processor = dasp_graph::Processor<Graph>;

fn get_sink_mono(g: &Graph, idx: NodeIndex<u32>) -> &CpalMonoSink {
    let n = g.index(idx);

    match &n.node {
        CpalMonoSink(s) => return s,
        _ => {
            panic!("i_out should definitely be a sink my guy.")
        }
    }
}

fn get_sink_stereo(g: &Graph, idx: NodeIndex<u32>) -> &CpalStereoSink {
    let n = g.index(idx);

    match &n.node {
        CpalStereoSink(s) => return s,
        _ => {
            panic!("i_out should definitely be a sink my guy.")
        }
    }
}

fn play_mono(p: &mut Processor, g: &mut Graph, endpoint: NodeIndex, secs: f32) {
    // for each second, process 48k samples / packet_size = num packets to process
    for _ in 0..(secs * (48000f32 / 64f32)) as usize {
        p.process(g, endpoint);
        let out = get_sink_mono(g, endpoint);

        let mut num_samples_queued = out.buffer.buffer().capacity() - out.buffer.slots();
        // try to leave at least 64 samples for cpal
        while num_samples_queued > 128 {
            sleep(Duration::from_micros(19*64));
            num_samples_queued = out.buffer.buffer().capacity() - out.buffer.slots();
        }
    }

    // let out = get_sink_mono(&g, endpoint);
    // sleep until the buffer is empty.
    // let mut num_samples_queued = out.buffer.buffer().capacity() - out.buffer.slots();
    // try to leave at least 64 samples for cpal
    // while num_samples_queued > 64 {
    //     sleep(Duration::from_micros(21*64));
    //     num_samples_queued = out.buffer.buffer().capacity() - out.buffer.slots();
    // }
}

fn play_stereo(p: &mut Processor, g: &mut Graph, endpoint: NodeIndex, secs: f32) {
    for _ in 0..(secs * (48000f32 / 128f32)) as usize {
        p.process(g, endpoint);

        let out = get_sink_stereo(g, endpoint);
        while out.buffer.slots() < 128 {
            sleep(Duration::from_micros(20*64));
        }
    }

    let out = get_sink_stereo(&g, endpoint);
    // sleep until the buffer is empty.
    while out.buffer.slots() < out.buffer.buffer().capacity() {
        sleep(Duration::from_micros(100));
    }
}

#[test]
/// Runs a 480hz sine wave at 48KHz for 5 seconds with mono out
fn sine_5s() {
    let sink = CpalMonoSink::default();
    let sine = Sine::new(SampleRate(48000), 960);

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_in = g.add_node(NodeData::new1(Sine(sine)));
    let i_out = g.add_node(NodeData::new(CpalMonoSink(sink), vec![]));
    g.add_edge(i_in, i_out, ());

    sleep(Duration::from_millis(500));

    let instant = Instant::now();
    play_mono(&mut p, &mut g, i_out, 5.0);
    println!("time: {:?}", instant.elapsed());
}

#[test]
/// Plays a 480Hz sine wave for 2.5s, followed by an additive 30Hz sine wave
fn sine_mix() {
    let sink = CpalMonoSink::default();
    let sine_c = Sine::new(SampleRate(48000), 131); // appx C2
    let sine_e = Sine::new(SampleRate(48000), 165); // appx E2
    let sine_g = Sine::new(SampleRate(48000), 196); // appx G2

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_c = g.add_node(NodeData::new1(Sine(sine_c)));
    let i_e = g.add_node(NodeData::new1(Sine(sine_e)));
    let i_g = g.add_node(NodeData::new1(Sine(sine_g)));
    let i_out = g.add_node(NodeData::new1(CpalMonoSink(sink)));
    let i_mix = g.add_node(NodeData::new1(NodeVariants::Sum(Sum)));
    let i_slew = g.add_node(NodeData::new1(NodeVariants::SlewLimiter(SlewLimiter::new())));
    let temp_edge = g.add_edge(i_c, i_mix, ());
    g.add_edge(i_mix, i_slew, ());
    g.add_edge(i_slew, i_out, ());



    sleep(Duration::from_millis(500));

    let instant = Instant::now();
    // play the first sine wave
    play_mono(&mut p, &mut g, i_out, 2.0);

    g.add_edge(i_c, i_mix, ());
    g.add_edge(i_e, i_mix, ());

    g.remove_edge(temp_edge);

    play_mono(&mut p, &mut g, i_out, 2.0);

    g.add_edge(i_g, i_mix, ());

    play_mono(&mut p, &mut g, i_out, 2.0);

    println!("time: {:?}", instant.elapsed());
}

#[test]
fn square_slewed() {
    let sink = CpalMonoSink::default();
    let sqr = Square::new(SampleRate(48000), 131);
    let slew = SlewLimiter::new();

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_in = g.add_node(NodeData::new1(Square(sqr)));
    let i_fx = g.add_node(NodeData::new1(SlewLimiter(slew)));
    let i_out = g.add_node(NodeData::new1(CpalMonoSink(sink)));
    let i_edge = g.add_edge(i_in, i_out, ());

    let instant = Instant::now();
    play_mono(&mut p, &mut g, i_out, 2.5);

    sleep(Duration::from_millis(500));

    g.remove_edge(i_edge);

    g.add_edge(i_in, i_fx, ());
    g.add_edge(i_fx, i_out, ());

    play_mono(&mut p, &mut g, i_out, 2.5);

    println!("time: {:?}", instant.elapsed());
}

#[test]
/// plays Low Tide - Wave of Colors in mono for 15 seconds.
/// Waves and birds should be audible in both ears.
fn vorbis_buffered() {
    let sink = CpalMonoSink::default();
    let mut ogg = None;

    if let Ok(o) = BufferedOgg::new(String::from("lowtide.ogg")) {
        ogg = Some(o);
    }

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_in = g.add_node(NodeData::new1(BufferedOgg(ogg.unwrap())));
    let i_out = g.add_node(NodeData::new1(CpalMonoSink(sink)));
    let i_edge = g.add_edge(i_in, i_out, ());

    let instant = Instant::now();
    play_mono(&mut p, &mut g, i_out, 15.0);

    println!("time: {:?}", instant.elapsed());
}

#[test]
/// plays Low Tide - Wave of Colors in stereo for 15 seconds.
/// Waves and birds should be audible mostly in the left ear.
fn vorbis_buffered2() {
    let sink = CpalStereoSink::default();
    let mut ogg = None;

    if let Ok(o) = BufferedOgg::new(String::from("lowtide.ogg")) {
        ogg = Some(o);
    }

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_in = g.add_node(NodeData::new2(BufferedOgg(ogg.unwrap())));
    let i_out = g.add_node(NodeData::new1(CpalStereoSink(sink)));
    let i_edge = g.add_edge(i_in, i_out, ());

    let instant = Instant::now();
    play_stereo(&mut p, &mut g, i_out, 15.0);

    println!("time: {:?}", instant.elapsed());
}

#[test]
fn stereo() {
    let sink = CpalStereoSink::default();
    let sine = Sine::new(SampleRate(48000), 131);

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_in = g.add_node(NodeData::new1(Sine(sine)));
    let i_mix = g.add_node(NodeData::new2(NodeVariants::Sum(Sum)));
    let i_out = g.add_node(NodeData::new1(CpalStereoSink(sink)));
    g.add_edge(i_in, i_mix, ());
    g.add_edge(i_mix, i_out, ());

    let instant = Instant::now();
    play_stereo(&mut p, &mut g, i_out, 5.0);

    println!("time: {:?}", instant.elapsed());
}
