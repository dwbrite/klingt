use cpal::SampleRate;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dasp_graph::{Buffer, Input, Node};

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("Sine.process()", |b| {
        let mut source = klingt::nodes::source::Sine::new(SampleRate(48000), 480);
        let mut output = [Buffer::default()];
        let input = [];

        b.iter(move || source.process(&input, &mut output))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
