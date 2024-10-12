use crate::AudioNode;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, SampleRate, Stream};
use dasp_graph::{Buffer, Input};
use rtrb::Producer;

pub struct CpalStereoSink {
    _stream: Stream,
    pub buffer: Producer<(f32, f32)>,
}

impl CpalStereoSink {
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
                .filter(|c| c.channels() == 2)
                .next().unwrap();

            fmt = cfg.sample_format();
            cfg.with_sample_rate(SampleRate(48000))
        };

        let config = supported_config.config();

        println!("config: {:?}", config);

        // TODO: figure out why the ringbuffer needs to be so large in order to consume audio fast enough.
        // stores two channels for ~10.67ms
        let (producer, mut consumer) = rtrb::RingBuffer::new(512);

        let channels = config.channels as usize;

        match fmt {
            SampleFormat::F32 => {
                let _stream = device
                    .build_output_stream::<f32, _, _>(
                        &config,
                        move |data, _| {
                            for chunk in data.chunks_mut(channels) {
                                let (l, r) = &consumer.pop().unwrap_or((0f32, 0f32));
                                for (i, d) in chunk.iter_mut().enumerate() {
                                    if i % 2 == 0 {
                                        *d = Sample::from_sample(*l);
                                    } else {
                                        *d = Sample::from_sample(*r);
                                    }
                                }
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
            _  => {println!("{fmt}"); todo!("more types")}
        }
    }
}

impl AudioNode for CpalStereoSink {
    fn process(&mut self, inputs: &[Input], _output: &mut [Buffer]) {
        if inputs.len() != 1 {
            panic!("a sink can only have one input. try mixing first.")
        }

        let stereo_channels = inputs.first().unwrap().buffers();
        for (&l, &r) in stereo_channels[0].iter().zip(stereo_channels[1].iter()) {
            self.buffer.push((l, r)).expect("👀");
        }

        self._stream.play().expect("smh");
    }
}
