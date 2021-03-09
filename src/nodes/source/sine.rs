use dasp_graph::{Buffer, Input, Node};

pub struct Sine {
    data: Vec<f32>,
    idx: usize,
}

impl Sine {
    pub fn new(sample_rate: cpal::SampleRate, frequency: u16) -> Sine {
        let cycle_time = 1.0 / frequency as f32;
        let total_samples = (sample_rate.0 as f32 * cycle_time) as usize;

        let mut data = Vec::<f32>::with_capacity(total_samples);

        for i in 0..total_samples {
            let pi = std::f32::consts::PI;
            let percent = (i as f32) / total_samples as f32;
            let rad_percent = percent * (2.0 * pi);
            let v = rad_percent.sin();

            data.push(v);
        }

        Sine { data, idx: 0 }
    }

    #[inline]
    fn next(&mut self) -> f32 {
        if self.idx >= self.data.len() {
            self.idx = 0;
        }
        self.data[self.idx]
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
