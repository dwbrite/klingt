use dasp_graph::{Buffer, Input, NodeData};
use klingt::nodes::sink::CpalStereoSink;
use klingt::nodes::source::BufferedOgg;
use klingt::AudioNode;
use petgraph::graph::NodeIndex;
use std::ops::Index;
use std::thread::sleep;
use std::time::{Duration, Instant};

use std::convert::TryInto;

type Graph = petgraph::graph::Graph<NodeData<NodeVariants>, ()>;
type Processor = dasp_graph::Processor<Graph>;

#[enum_delegate::implement(AudioNode,
    pub trait AudioNode {
        fn process(&mut self, inputs: &[Input], output: &mut [Buffer]);
    }
)]
pub enum NodeVariants {
    CpalStereoSink(CpalStereoSink),
    BufferedOgg(BufferedOgg),
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

fn main() {
    let sink = CpalStereoSink::default();
    let mut ogg = None;

    if let Ok(o) = BufferedOgg::new(String::from("lowtide.ogg")) {
        ogg = Some(o);
    }

    let mut g = Graph::with_capacity(64, 64);
    let mut p = Processor::with_capacity(64);

    let i_in = g.add_node(NodeData::new2(NodeVariants::BufferedOgg(ogg.unwrap())));
    let i_out = g.add_node(NodeData::new1(NodeVariants::CpalStereoSink(sink)));
    let i_edge = g.add_edge(i_in, i_out, ());

    let instant = Instant::now();
    play_stereo(&mut p, &mut g, i_out, 15.0);

    println!("time: {:?}", instant.elapsed());
}
