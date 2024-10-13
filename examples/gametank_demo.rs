use core::convert::TryInto;
use std::ops::{Index, IndexMut};
use std::{thread};
use std::collections::VecDeque;


use std::thread::sleep;
use std::time::Duration;
use cpal::BufferSize::Fixed;
use cpal::SampleRate;
use dasp_frame::Frame;
use dasp_graph::{Buffer, Input, NodeData};
use dasp_interpolate::linear::Linear;
use dasp_interpolate::sinc::Sinc;
use dasp_ring_buffer::Slice;
use dasp_sample::FromSample;
use dasp_signal::interpolate::Converter;
use dasp_signal::{rate, Signal};
use petgraph::graph::NodeIndex;
use rtrb::{Consumer, Producer, RingBuffer};
use tracing::{event, instrument, Level, trace, trace_span};


use klingt::{AudioNode, Klingt};
use klingt::nodes::sink::CpalMonoSink;
use klingt::nodes::effect::SlewLimiter;

use itertools::*;

pub struct GameTankSignal {
    buffer: Consumer<u8>,
}

impl GameTankSignal {
    pub fn new(buffer: Consumer<u8>) -> Self {
        Self {
            buffer,
        }
    }
}

impl Signal for GameTankSignal {
    type Frame = f32;

    fn next(&mut self) -> Self::Frame {
        if let Ok(sample) = self.buffer.pop() {
            (sample as f32 / 255.0) * 2.0 - 1.0
        } else {
            println!("FEED THE BUFFFEERRRRRR");
            0.0
        }
    }

    fn is_exhausted(&self) -> bool {
        self.buffer.slots() < 64
    }
}


#[derive(Debug)]
pub struct RtrbSource {
    output_buffer: Consumer<Buffer>
}

#[derive()]
pub struct GameTankAudio {
    input_producer: Producer<u8>,
    resampled: VecDeque<f32>,

    output_queue: Producer<Buffer>, // ring buffer for output buffers

    sample_rate: f64,
    target_sample_rate: f64,
    converter: Box<dyn Signal<Frame = f32> + Send>,
}

impl GameTankAudio {
    pub fn new(sample_rate: f64, target_sample_rate: f64) -> (Self, Consumer<Buffer>) {
        let (input_producer, input_buffer) = RingBuffer::<u8>::new(512); // Ring buffer to hold GameTank samples
        let (output_producer, output_consumer) = RingBuffer::<Buffer>::new(32); // Ring buffer to hold output buffers
        let interp = Linear::new(0.0, 0.0);
        let mut signal = GameTankSignal::new(input_buffer);
        let converter = signal.from_hz_to_hz(interp, sample_rate, target_sample_rate);

        (
            Self {
                input_producer,
                resampled: VecDeque::with_capacity(1024),
                output_queue: output_producer,
                sample_rate,
                target_sample_rate,
                converter: Box::new(converter),
            },
            output_consumer
        )
    }

    pub fn convert_to_output_buffers(&mut self) {
        while !self.converter.is_exhausted() {
            self.resampled.push_back(self.converter.next());
        }

        while self.resampled.len() >= 64 {
            if let Some(chunk) = self.resampled.drain(..64).collect::<Vec<_>>().try_into().ok() {
                let mut buf = Buffer::SILENT;
                for (b, v) in buf.iter_mut().zip::<[f32;64]>(chunk) {
                    *b = v;
                }
                self.output_queue.push(buf).unwrap()
            }
        }

    }
}

impl AudioNode for RtrbSource {
    #[instrument]
    fn process(&mut self, _inputs: &[Input], output: &mut [Buffer]) {
        let b = match self.output_buffer.pop() {
            Ok(buf) => { buf }
            Err(_) => { println!("FEED THE BUFFER"); Buffer::SILENT }
        };
        for buffer in output.iter_mut() {
            *buffer = b.clone();
        }
        event!(Level::INFO, "processed rtrb source");
    }
}

#[enum_delegate::implement(AudioNode, pub trait AudioNode { fn process(&mut self, inputs: &[Input], output: &mut [Buffer]);})]
pub enum GTNode {
    CpalMonoSink(CpalMonoSink),
    GameTankSource(RtrbSource),
    SlewLimiter(SlewLimiter)
}

fn get_sink_mono(g: &klingt::KlingtGraph<GTNode>, idx: NodeIndex<u32>) -> &CpalMonoSink {
    let n = g.index(idx);

    match &n.node {
        GTNode::CpalMonoSink(s) => return &s,
        _ => {
            panic!("i_out should definitely be a sink my guy.")
        }
    }
}

fn main() {
    let mut klingt = Klingt::<GTNode>::default();

    let sink = CpalMonoSink::default();
    let slew = NodeData::new1(GTNode::SlewLimiter(SlewLimiter::new()));
    let out_node = NodeData::new1(GTNode::CpalMonoSink(sink));

    let sample_rate = 13982.95;
    let target_sample_rate = 48000.0;
    let (mut gta, gta_output) = GameTankAudio::new(sample_rate, target_sample_rate);

    let gt_node = NodeData::new1(GTNode::GameTankSource(RtrbSource{ output_buffer: gta_output }));

    let idx_out = klingt.add_node(out_node);
    let idx_in = klingt.add_node(gt_node);

    let idx_fx = klingt.add_node(slew);
    klingt.add_edge(idx_in, idx_fx, ());
    klingt.add_edge(idx_fx, idx_out, ());


    // Generate a 130Hz sine wave at 13,982.95 Hz sample rate
    let mut sine_wave = rate(sample_rate).const_hz(60.0).sine();

    thread::spawn(move || {
        let _ = trace_span!("gta loop").enter();
        loop {
            // if it's (nearly) empty, add 256 more samples
            if gta.converter.is_exhausted() {
                for _ in 0..256 {
                    let next_sample_u8 = (( sine_wave.next() + 1.0) / 2.0 * 255.0) as u8;
                    gta.input_producer.push(next_sample_u8).expect("failure.");
                }
            }
            gta.convert_to_output_buffers();

            // println!(">>\t1.) produced inputs");
            trace!("produced 256 inputs, waiting for available slots");

            // slots available for writing
            // we want to wait until there are: 256*~3.5/64=~14 slots available? round to 16
            // so, if there are 16 slots available, then we can create and insert 256 samples into the input buffer, and then we wait...
            while gta.output_queue.slots() < 16 {
                // println!("{}", gta.output_producer.slots());
                sleep(Duration::from_micros(200))
            }
            trace!("waiting complete, buffers available");
        }
    });


    let mut ready_to_output = 0;
    let _ = trace_span!("graph processing loop").enter();
    loop {
        // Calculate the number of buffers to generate since the last frame
        if let GTNode::GameTankSource(src) = &mut klingt.index_mut(idx_in).node {
            ready_to_output = src.output_buffer.slots();
        }

        // Generate buffers in a loop
        let mut can_output = get_sink_mono(&klingt, idx_out).buffer.slots() >= 64 && ready_to_output >= 4;

        while can_output {
            klingt.processor.process(&mut klingt.graph, idx_out);

            if let GTNode::GameTankSource(src) = &mut klingt.index_mut(idx_in).node {
                ready_to_output = src.output_buffer.slots();
                can_output = get_sink_mono(&klingt, idx_out).buffer.slots() >= 64 && ready_to_output >= 4;
                sleep(Duration::from_millis(1)); // takes 1.33ms per 64 samples, so this should be safe
                trace!("ready to output {ready_to_output}");
            }
        }
    }
}