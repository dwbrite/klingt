use cpal::SampleRate;
use dasp_graph::node::Sum;
use dasp_graph::NodeData;
use klingt::nodes::effect::SlewLimiter;
use klingt::nodes::sink::{CpalMonoSink, CpalStereoSink};
use klingt::nodes::source::{BufferedOgg, Sine, Square};
use klingt::NodeVariants;
use petgraph::prelude::NodeIndex;
use std::ops::Index;
use std::thread::sleep;
use std::time::{Duration, Instant};

type Graph = petgraph::graph::Graph<NodeData<NodeVariants>, ()>;
type Processor = dasp_graph::Processor<Graph>;

fn get_sink_mono(g: &Graph, idx: NodeIndex<u32>) -> &CpalMonoSink {
    let n = g.index(idx);

    match &n.node {
        NodeVariants::CpalMonoSink(s) => return s,
        _ => {
            panic!("i_out should definitely be a sink my guy.")
        }
    }
}

fn get_sink_stereo(g: &Graph, idx: NodeIndex<u32>) -> &CpalStereoSink {
    let n = g.index(idx);

    match &n.node {
        NodeVariants::CpalStereoSink(s) => return s,
        _ => {
            panic!("i_out should definitely be a sink my guy.")
        }
    }
}

fn play_mono(p: &mut Processor, g: &mut Graph, endpoint: NodeIndex, secs: f32) {
    for _ in 0..(secs * (48000 as f32 / 64 as f32)) as usize {
        p.process(g, endpoint);

        let out = get_sink_mono(g, endpoint);
        while out.buffer.slots() < 128 {
            sleep(Duration::from_micros(400));
        }
    }

    let out = get_sink_mono(&g, endpoint);
    // sleep until the buffer is empty.
    while out.buffer.slots() < out.buffer.buffer().capacity() {
        sleep(Duration::from_micros(100));
    }
}

fn play_stereo(p: &mut Processor, g: &mut Graph, endpoint: NodeIndex, secs: f32) {
    for _ in 0..(secs * (48000 as f32 / 64 as f32)) as usize {
        p.process(g, endpoint);

        let out = get_sink_stereo(g, endpoint);
        while out.buffer.slots() < 128 {
            sleep(Duration::from_micros(400));
        }
    }

    let out = get_sink_stereo(&g, endpoint);
    // sleep until the buffer is empty.
    while out.buffer.slots() < out.buffer.buffer().capacity() {
        sleep(Duration::from_micros(100));
    }
}

#[test]
fn sine_5s() {
    let sink = CpalMonoSink::default();
    let sine = Sine::new(SampleRate(48000), 480);

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_in = g.add_node(NodeData::new2(sine.into()));
    let i_out = g.add_node(NodeData::new(sink.into(), vec![]));
    g.add_edge(i_in, i_out, ());

    sleep(Duration::from_millis(500));

    let instant = Instant::now();
    play_mono(&mut p, &mut g, i_out, 5.0);
    println!("time: {:?}", instant.elapsed());
}

#[test]
fn sine_mix() {
    let sink = CpalMonoSink::default();
    let sine_a = Sine::new(SampleRate(48000), 480);
    let sine_b = Sine::new(SampleRate(48000), 690);

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_a = g.add_node(NodeData::new1(sine_a.into()));
    let i_b = g.add_node(NodeData::new1(sine_b.into()));
    let i_out = g.add_node(NodeData::new1(sink.into()));
    let i_mix = g.add_node(NodeData::new1(Sum.into()));
    let temp_edge = g.add_edge(i_a, i_out, ());

    sleep(Duration::from_millis(500));

    let instant = Instant::now();
    // play the first sine wave
    play_mono(&mut p, &mut g, i_out, 2.5);

    g.add_edge(i_a, i_mix, ());
    g.add_edge(i_b, i_mix, ());
    g.add_edge(i_mix, i_out, ());

    g.remove_edge(temp_edge);

    play_mono(&mut p, &mut g, i_out, 2.5);

    println!("time: {:?}", instant.elapsed());
}

#[test]
fn square_slewed() {
    let sink = CpalMonoSink::default();
    let sqr = Square::new(SampleRate(48000), 480);
    let slew = SlewLimiter::new();

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_in = g.add_node(NodeData::new1(sqr.into()));
    let i_fx = g.add_node(NodeData::new1(slew.into()));
    let i_out = g.add_node(NodeData::new1(sink.into()));
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
fn vorbis_buffered() {
    let sink = CpalMonoSink::default();
    let mut ogg = None;

    if let Ok(o) = BufferedOgg::new(String::from("lowtide.ogg")) {
        ogg = Some(o);
    }

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_in = g.add_node(NodeData::new1(ogg.unwrap().into()));
    let i_out = g.add_node(NodeData::new1(sink.into()));
    let i_edge = g.add_edge(i_in, i_out, ());

    let instant = Instant::now();
    play_mono(&mut p, &mut g, i_out, 100.0);

    println!("time: {:?}", instant.elapsed());
}

#[test]
fn vorbis_buffered2() {
    let sink = CpalStereoSink::default();
    let mut ogg = None;

    if let Ok(o) = BufferedOgg::new(String::from("lowtide.ogg")) {
        ogg = Some(o);
    }

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_in = g.add_node(NodeData::new2(ogg.unwrap().into()));
    let i_out = g.add_node(NodeData::new1(sink.into()));
    let i_edge = g.add_edge(i_in, i_out, ());

    let instant = Instant::now();
    play_stereo(&mut p, &mut g, i_out, 15.0);

    println!("time: {:?}", instant.elapsed());
}

#[test]
fn stereo() {
    let sink = CpalStereoSink::default();
    let sine = Sine::new(SampleRate(48000), 480);

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_in = g.add_node(NodeData::new1(sine.into()));
    let i_mix = g.add_node(NodeData::new2(Sum.into()));
    let i_out = g.add_node(NodeData::new1(sink.into()));
    g.add_edge(i_in, i_mix, ());
    g.add_edge(i_mix, i_out, ());

    let instant = Instant::now();
    play_stereo(&mut p, &mut g, i_out, 5.0);

    println!("time: {:?}", instant.elapsed());
}
