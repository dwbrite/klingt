use std::io;
use std::io::Write;
use crate::AudioNode;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{SampleFormat, SampleRate, Stream};
use dasp_graph::{Buffer, Input};
use rtrb::{Producer};


pub struct CpalMonoSink {
    stream: Stream,
    pub buffer: Producer<f32>,
}

impl CpalMonoSink {
    pub fn default() -> Self {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .expect("no output device available");

        println!("device: {:?}", device.name().unwrap());

        let supported_configs_range = device
            .supported_output_configs()
            .expect("error while querying configs");

        let fmt;
        let supported_config = {
            let cfg = supported_configs_range
                .filter(|c| c.sample_format() == SampleFormat::F32)
                .filter(|c| c.channels() == 1)
                .next().unwrap();
            fmt = cfg.sample_format();
            cfg.with_sample_rate(SampleRate(48000))
        };

        let mut config = supported_config.config();
        config.buffer_size = cpal::BufferSize::Fixed(256);

        println!("config: {:?}", config);
        // holds 512 samples, which should be about ~5 milliseconds.
        let (producer, mut consumer) = rtrb::RingBuffer::new(512);

        let channels = config.channels as usize;

        let _stream = device
            .build_output_stream::<f32, _, _>(
                &config,
                move |data, _| {
                    let _data_size = data.len();
                    for chunk in data.chunks_mut(channels) {
                        let s = match consumer.pop() {
                            Ok(v) => {
                                // println!("{}/{data_size} samples left in pre-output buffer", consumer.slots());
                                v * 0.2
                            }
                            Err(_) => {
                                // error!("{}/{data_size} samples left in pre-output buffer", consumer.slots());
                                0.0f32
                            }
                        };
                        let v = cpal::Sample::from_sample(s);
                        chunk.iter_mut().for_each(|d| {
                            // Self::print_waveline(v);
                            *d = v;
                        })
                    }
                },
                move |err| {
                    println!("{:?}", err);
                },
                None
            )
            .expect("you were fucked from the start.");
        Self {
            stream: _stream,
            buffer: producer,
        }
    }

    fn print_waveline(v: f32) {
        let max = ((v + 1.0) * 5.0) as u32;
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        for i in 0..=max {
            if i == max {
                if max == 5 {
                    handle.write(b"!");
                } else {
                    handle.write(b".");
                }
            } else {
                handle.write(b" ");
            }
        }
        handle.write(b"\n");
    }
}

impl AudioNode for CpalMonoSink {
    fn process(&mut self, inputs: &[Input], _output: &mut [Buffer]) {
        if inputs.len() != 1 {
            panic!("a sink can only have one input. try mixing first.")
        }

        let mono_channel = inputs.first().unwrap().buffers().first().unwrap();
        for &sample in mono_channel.iter() {
            // Self::print_waveline(sample);
            match self.buffer.push(sample) {
                Ok(_) => {}
                Err(_) => {
                    println!("couldn't write to output buffer: {} of {} slots available", self.buffer.slots(), self.buffer.buffer().capacity());
                }
            }
        }
    }
}
