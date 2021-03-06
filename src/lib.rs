use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{BufferSize, SampleRate, Stream};
use dasp_graph::{Buffer, Input, Node};
use rtrb::Producer;
use std::collections::VecDeque;

#[cfg(test)]
mod tests {
    use crate::IO;
    use cpal::SampleRate;
    use dasp_graph::NodeData;
    use std::ops::IndexMut;
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    type Graph = petgraph::graph::Graph<NodeData<IO>, ()>;
    type Processor = dasp_graph::Processor<Graph>;

    #[test]
    fn sine_5s() {
        let sink = crate::Sink::default();
        let sine = crate::Sine::new(SampleRate(48000), 480);

        let mut g = Graph::with_capacity(64, 64);
        let mut p = Processor::with_capacity(64);

        let i_in = g.add_node(NodeData::new1(IO::Sine(sine)));
        let i_out = g.add_node(NodeData::new1(IO::Sink(sink)));
        g.add_edge(i_in, i_out, ());

        sleep(Duration::from_millis(500));

        let instant = Instant::now();

        // play for 5 seconds
        for _ in 0..(5 * (48000 / 64)) {
            p.process(&mut g, i_out);

            let n = g.index_mut(i_out);
            match &mut n.node {
                IO::Sink(s) => {
                    while s.buffer.slots() < 64 {
                        sleep(Duration::from_micros(400));
                    }
                }
                IO::Sine(_) => {}
            }
        }

        println!("time: {:?}", instant.elapsed());

        assert_eq!(2 + 2, 4);
    }
}

// TODO: docs, explain how you can do this too at a higher level
enum IO {
    Sink(Sink),
    Sine(Sine),
}

impl Node for IO {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        match self {
            IO::Sink(s) => s.process(inputs, output),
            IO::Sine(s) => s.process(inputs, output),
        }
    }
}

pub struct Sink {
    _stream: Stream,
    buffer: Producer<f32>,
}

impl Sink {
    pub fn default() -> Self {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");

        let mut supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");
        let supported_config = {
            let mut config = None;
            for cfg in supported_configs_range {
                if cfg.channels() != 1 {
                    continue;
                }

                if let cpal::SupportedBufferSize::Range { min, max } = *cfg.buffer_size() {
                    if min <= 64 && max >= 64 {
                        config = Some(cfg);
                    }
                }
            }
            config.unwrap().with_sample_rate(SampleRate(48000))
        };

        let config = supported_config.config();

        println!("config: {:?}", config);

        // TODO: figure out why the ringbuffer needs to be so large in order to consume audio fast enough.
        // try using chunks with a smaller ringbuffer?
        let (producer, mut consumer) = rtrb::RingBuffer::new(4096).split();

        let stream = device
            .build_output_stream::<f32, _, _>(
                &config,
                move |data, _| {
                    data.iter_mut().for_each(|d| {
                        *d = consumer.pop().unwrap_or(0f32);
                    });
                },
                move |err| {
                    println!("{:?}", err);
                },
            )
            .expect("you were fucked from the start.");

        Self {
            _stream: stream,
            buffer: producer,
        }
    }
}

impl Node for Sink {
    fn process(&mut self, inputs: &[Input], _output: &mut [Buffer]) {
        if inputs.len() != 1 {
            panic!("a sink can only have one input. try mixing first.")
        }

        for buffer in inputs.first().unwrap().buffers() {
            for &sample in buffer.iter() {
                self.buffer.push(sample).expect("👀");
            }
        }
    }
}

pub struct Sine {
    data: VecDeque<f32>,
}

impl Sine {
    fn new(sample_rate: cpal::SampleRate, frequency: u16) -> Sine {
        let cycle_time = 1.0 / frequency as f32;
        let total_samples = (sample_rate.0 as f32 * cycle_time) as usize;

        let mut data = VecDeque::<f32>::with_capacity(total_samples);

        for i in 0..total_samples {
            let pi = std::f32::consts::PI;
            let percent = (i as f32) / total_samples as f32;
            let rad_percent = percent * (2.0 * pi);
            let v = rad_percent.sin();

            data.push_back(v);
        }

        Sine { data }
    }

    #[inline]
    fn next(&mut self) -> f32 {
        let a = self.data.pop_front().unwrap();
        self.data.push_back(a);
        a
    }
}

impl Node for Sine {
    fn process(&mut self, _: &[Input], output: &mut [Buffer]) {
        for buffer in output.iter_mut() {
            for sample in buffer.iter_mut() {
                *sample = self.next();
            }
        }
    }
}

// TODO: benchmark

// TODO: buffered sources, mixing, seeking (on seekable sources?), {mono/stereo/3d audio, hrtf, doppler}
