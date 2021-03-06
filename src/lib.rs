use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, Stream};
use dasp_graph::{Buffer, Input, Node};
use rtrb::Producer;
use std::collections::VecDeque;
use std::iter::Sum;

#[cfg(test)]
mod tests {
    use crate::{Sink, IO};
    use cpal::SampleRate;
    use dasp_graph::node::Sum;
    use dasp_graph::NodeData;
    use petgraph::prelude::NodeIndex;
    use std::ops::Index;
    use std::thread::sleep;
    use std::time::{Duration, Instant};

    type Graph = petgraph::graph::Graph<NodeData<IO>, ()>;
    type Processor = dasp_graph::Processor<Graph>;

    fn get_sink(g: &Graph, idx: NodeIndex<u32>) -> &Sink {
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
        let sink = crate::Sink::default();
        let sine = crate::Sine::new(SampleRate(48000), 480);

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
        let sink = crate::Sink::default();
        let sine_a = crate::Sine::new(SampleRate(48000), 480);
        let sine_b = crate::Sine::new(SampleRate(48000), 690);

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

        assert_eq!(2 + 2, 4);
    }
}

// TODO: docs, explain how you can do this too at a higher level
pub enum IO {
    Sink(Sink),
    Sine(Sine),
    Sum(dasp_graph::node::Sum),
}

impl Node for IO {
    fn process(&mut self, inputs: &[Input], output: &mut [Buffer]) {
        match self {
            IO::Sink(s) => s.process(inputs, output),
            IO::Sine(s) => s.process(inputs, output),
            IO::Sum(s) => s.process(inputs, output),
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

        println!("device: {:?}", device.name().unwrap());

        let mut supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");

        let fmt;
        let supported_config = {
            let cfg = supported_configs_range.next().unwrap();

            fmt = cfg.sample_format();
            cfg.with_sample_rate(SampleRate(48000))
        };

        let config = supported_config.config();

        println!("config: {:?}", config);

        // TODO: figure out why the ringbuffer needs to be so large in order to consume audio fast enough.
        // try using chunks with a smaller ringbuffer?
        let (producer, mut consumer) = rtrb::RingBuffer::new(4096).split();

        let channels = config.channels as usize;

        match fmt {
            SampleFormat::I16 => {
                let _stream = device
                    .build_output_stream::<i16, _, _>(
                        &config,
                        move |data, _| {
                            for chunk in data.chunks_mut(channels) {
                                let v = cpal::Sample::from(&consumer.pop().unwrap_or(0f32));
                                chunk.iter_mut().for_each(|d| {
                                    *d = v;
                                })
                            }
                        },
                        move |err| {
                            println!("{:?}", err);
                        },
                    )
                    .expect("you were fucked from the start.");

                Self {
                    _stream,
                    buffer: producer,
                }
            }
            SampleFormat::U16 => {
                let _stream = device
                    .build_output_stream::<u16, _, _>(
                        &config,
                        move |data, _| {
                            for chunk in data.chunks_mut(channels) {
                                let v = cpal::Sample::from(&consumer.pop().unwrap_or(0f32));
                                chunk.iter_mut().for_each(|d| {
                                    *d = v;
                                })
                            }
                        },
                        move |err| {
                            println!("{:?}", err);
                        },
                    )
                    .expect("you were fucked from the start.");

                Self {
                    _stream,
                    buffer: producer,
                }
            }
            SampleFormat::F32 => {
                let _stream = device
                    .build_output_stream::<f32, _, _>(
                        &config,
                        move |data, _| {
                            for chunk in data.chunks_mut(channels) {
                                let v = cpal::Sample::from(&consumer.pop().unwrap_or(0f32));
                                chunk.iter_mut().for_each(|d| {
                                    *d = v;
                                })
                            }
                        },
                        move |err| {
                            println!("{:?}", err);
                        },
                    )
                    .expect("you were fucked from the start.");
                Self {
                    _stream,
                    buffer: producer,
                }
            }
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
        self._stream.play().expect("smh");
    }
}

pub struct Sine {
    data: VecDeque<f32>,
}

impl Sine {
    pub fn new(sample_rate: cpal::SampleRate, frequency: u16) -> Sine {
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

// TODO: buffered sources, seeking (on seekable sources?), {mono/stereo/3d audio, hrtf, doppler}
