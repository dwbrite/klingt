use dasp_graph::{Buffer, Input, NodeData};
use klingt::{AudioNode, NodeVariants};
use petgraph::graph::NodeIndex;
use std::ops::Index;
use std::thread::sleep;
use std::time::{Duration, Instant};

use std::convert::TryInto;
use klingt::nodes::sink::CpalStereoSink;
use klingt::nodes::source::BufferedOgg;

#[enum_delegate::implement(AudioNode, pub trait Node { fn process(&mut self, inputs: &[Input], output: &mut [Buffer]);})]
pub enum MyNode {
    CpalStereoSink(CpalStereoSink),
    BufferedOgg(BufferedOgg),
    NodeVariants(NodeVariants),
}

type Klingt = klingt::Klingt<MyNode>;

fn get_sink_stereo(klingt: &Klingt, idx: NodeIndex<u32>) -> &CpalStereoSink {
    let n = klingt.index(idx);

    match &n.node {
        MyNode::CpalStereoSink(s) => return s,
        _ => {
            panic!("i_out should definitely be a sink my guy.")
        }
    }
}

fn play_stereo(klingt: &mut Klingt, endpoint: NodeIndex, secs: f32) {
    for _ in 0..(secs * (48000 as f32 / 64 as f32)) as usize {
        klingt.process_to_idx(endpoint);

        let out = get_sink_stereo(klingt, endpoint);
        while out.buffer.slots() < 128 {
            sleep(Duration::from_micros(400));
        }
    }

    let out = get_sink_stereo(&klingt, endpoint);
    // sleep until the buffer is empty.
    while out.buffer.slots() < out.buffer.buffer().capacity() {
        sleep(Duration::from_micros(100));
    }
}

fn main() {
    let mut klingt = Klingt::default();

    let sink = CpalStereoSink::default();
    let mut ogg = BufferedOgg::new(String::from("lowtide.ogg")).unwrap();

    let i_in = klingt.add_node(NodeData::new2(MyNode::BufferedOgg(ogg)));
    let i_out = klingt.add_node(NodeData::new1(MyNode::from(sink)));
    let _i_edge = klingt.add_edge(i_in, i_out, ());

    let instant = Instant::now();
    play_stereo(&mut klingt, i_out, 15.0);

    println!("time: {:?}", instant.elapsed());
}
