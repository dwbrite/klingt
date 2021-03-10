use cpal::SampleRate;
use dasp_graph::node::Sum;
use dasp_graph::NodeData;
use klingt::nodes::effect::SlewLimiter;
use klingt::nodes::sink::CpalMonoSink;
use klingt::nodes::source::{Sine, Square};
use klingt::IO;
use petgraph::prelude::NodeIndex;
use std::ops::Index;
use std::thread::sleep;
use std::time::{Duration, Instant};

type Graph = petgraph::graph::Graph<NodeData<IO>, ()>;
type Processor = dasp_graph::Processor<Graph>;

fn get_sink(g: &Graph, idx: NodeIndex<u32>) -> &CpalMonoSink {
    let n = g.index(idx);

    match &n.node {
        IO::Sink(s) => return s,
        _ => {
            panic!("i_out should definitely be a sink my guy.")
        }
    }
}

fn play(p: &mut Processor, g: &mut Graph, endpoint: NodeIndex, secs: f32) {
    for _ in 0..(secs * (48000 as f32 / 64 as f32)) as usize {
        p.process(g, endpoint);

        let out = get_sink(g, endpoint);
        while out.buffer.slots() < 64 {
            sleep(Duration::from_micros(400));
        }
    }

    let out = get_sink(&g, endpoint);
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

    let i_in = g.add_node(NodeData::new1(IO::Sine(sine)));
    let i_out = g.add_node(NodeData::new1(IO::Sink(sink)));
    g.add_edge(i_in, i_out, ());

    sleep(Duration::from_millis(500));

    let instant = Instant::now();
    play(&mut p, &mut g, i_out, 5.0);
    println!("time: {:?}", instant.elapsed());
}

#[test]
fn sine_mix() {
    let sink = CpalMonoSink::default();
    let sine_a = Sine::new(SampleRate(48000), 480);
    let sine_b = Sine::new(SampleRate(48000), 690);

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_a = g.add_node(NodeData::new1(IO::Sine(sine_a)));
    let i_b = g.add_node(NodeData::new1(IO::Sine(sine_b)));
    let i_out = g.add_node(NodeData::new1(IO::Sink(sink)));
    let i_mix = g.add_node(NodeData::new1(IO::Sum(Sum)));
    let temp_edge = g.add_edge(i_a, i_out, ());

    sleep(Duration::from_millis(500));

    let instant = Instant::now();
    // play the first sine wave
    play(&mut p, &mut g, i_out, 2.5);

    g.add_edge(i_a, i_mix, ());
    g.add_edge(i_b, i_mix, ());
    g.add_edge(i_mix, i_out, ());

    g.remove_edge(temp_edge);

    play(&mut p, &mut g, i_out, 2.5);

    println!("time: {:?}", instant.elapsed());
}

#[test]
fn square_slewed() {
    let sink = CpalMonoSink::default();
    let sqr = Square::new(SampleRate(48000), 480);
    let slew = SlewLimiter::new();

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_in = g.add_node(NodeData::new1(IO::Square(sqr)));
    let i_fx = g.add_node(NodeData::new1(IO::SlewLim(slew)));
    let i_out = g.add_node(NodeData::new1(IO::Sink(sink)));
    let i_edge = g.add_edge(i_in, i_out, ());

    let instant = Instant::now();
    play(&mut p, &mut g, i_out, 2.5);

    sleep(Duration::from_millis(500));

    g.remove_edge(i_edge);

    g.add_edge(i_in, i_fx, ());
    g.add_edge(i_fx, i_out, ());

    play(&mut p, &mut g, i_out, 2.5);

    println!("time: {:?}", instant.elapsed());
}
