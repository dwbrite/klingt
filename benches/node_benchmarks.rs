use cpal::SampleRate;
use criterion::{criterion_group, criterion_main, Criterion};
use dasp_graph::{Buffer, Node, NodeData};
use klingt::nodes::effect::SlewLimiter;
use klingt::nodes::source::Sine;

use klingt::AudioNode;
use klingt::NodeVariants::*;

type Graph = petgraph::graph::Graph<NodeData<klingt::NodeVariants>, ()>;
type Processor = dasp_graph::Processor<Graph>;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("Sine.process()", |b| {
        let mut source = Sine::new(SampleRate(48000), 480);
        let mut output = [Buffer::default()];
        let input = [];

        b.iter(move || source.process_inner(&input, &mut output))
    });

    c.bench_function("Sine.process(), integrated", |b| {
        let sine = Sine::new(SampleRate(48000), 480);
        let _slewlimiter = SlewLimiter::new();

        let mut g = Graph::with_capacity(64, 64);
        let mut p = Processor::with_capacity(64);

        let i_in = g.add_node(NodeData::new1(Sine(sine)));

        b.iter(move || p.process(&mut g, i_in))
    });

    c.bench_function("SlewLimiter.process()", |b| {
        let sine = Sine::new(SampleRate(48000), 480);
        let slewlimiter = SlewLimiter::new();

        let mut g = Graph::with_capacity(64, 64);
        let mut p = Processor::with_capacity(64);

        let i_in = g.add_node(NodeData::new1(Sine(sine)));
        let i_out = g.add_node(NodeData::new1(SlewLimiter(slewlimiter)));
        g.add_edge(i_in, i_out.clone(), ());

        b.iter(move || p.process(&mut g, i_out))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
