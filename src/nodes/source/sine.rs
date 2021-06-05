use crate::AudioNode;
use dasp_graph::{Buffer, Input, Node};
use std::collections::VecDeque;

pub struct Sine {
    data: Vec<f32>,
    idx: usize,
}

impl Sine {
    pub fn new(sample_rate: cpal::SampleRate, frequency: u16) -> Sine {
        let cycle_time = 1.0 / frequency as f32;
        let total_samples = (sample_rate.0 as f32 * cycle_time) as usize;

        let mut data = Vec::<f32>::new();

        for i in 0..total_samples {
            let pi = std::f32::consts::PI;
            let percent = (i as f32) / total_samples as f32;
            let rad_percent = percent * (2.0 * pi);
            let v = rad_percent.sin();

            data.push(v);
        }

        Sine { data, idx: 0 }
    }
}

impl Iterator for Sine {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        match self.data.get(self.idx) {
            Some(v) => {
                self.idx += 1;
                Some(*v)
            }
            None => {
                self.idx = 0;
                self.next()
            }
        }
    }
}

impl AudioNode for Sine {
    fn process(&mut self, _: &[Input], output: &mut [Buffer]) {
        for buffer in output.iter_mut() {
            for sample in buffer.iter_mut() {
                *sample = self.next().unwrap();
            }
        }
    }
}
