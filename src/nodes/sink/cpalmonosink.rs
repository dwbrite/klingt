use crate::AudioNode;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, Stream};
use dasp_graph::{Buffer, Input};
use rtrb::Producer;

pub struct CpalMonoSink {
    _stream: Stream,
    pub buffer: Producer<f32>,
}

impl CpalMonoSink {
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
                                let v = <dyn cpal::Sample>::from(&consumer.pop().unwrap_or(0f32));
                                chunk.iter_mut().for_each(|d| {
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
                                let v = <dyn cpal::Sample>::from(&consumer.pop().unwrap_or(0f32));
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
                                let v = <dyn cpal::Sample>::from(&consumer.pop().unwrap_or(0f32));
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

impl AudioNode for CpalMonoSink {
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
