use crate::nodes::source::BufferedOggError::{IoErr, VorbisErr};
use crate::AudioNode;
use dasp_graph::{Buffer, Input, Node};
use lewton::inside_ogg::OggStreamReader;
use lewton::VorbisError;
use std::collections::VecDeque;
use std::fs::File;
use std::io::Error;
use std::path::Path;
use std::rc::Rc;

pub struct BufferedOgg {
    data: Rc<[Vec<f32>; 2]>,
    idx: [usize; 2],
    channels: usize,
}

pub enum BufferedOggError {
    VorbisErr(VorbisError),
    IoErr(std::io::Error),
}

impl From<std::io::Error> for BufferedOggError {
    fn from(e: Error) -> Self {
        IoErr(e)
    }
}

impl From<VorbisError> for BufferedOggError {
    fn from(e: VorbisError) -> Self {
        VorbisErr(e)
    }
}

impl BufferedOgg {
    pub fn new(file_path: String) -> Result<BufferedOgg, BufferedOggError> {
        let f = File::open(file_path)?;
        let mut srr = OggStreamReader::new(f)?;

        // TODO: convert sample rates

        let mut data = [vec![], vec![]];

        while let Some(pck_samples) = srr.read_dec_packet_itl()? {
            for (idx, sample) in pck_samples.iter().enumerate() {
                data[idx % 2].push(cpal::Sample::from(sample));
            }
        }

        println!("done loading :^)");

        Ok(BufferedOgg {
            data: Rc::new(data),
            idx: [0, 0],
            channels: 2,
        })
    }

    fn next(&mut self, channel: usize) -> f32 {
        let v = self.data[channel][self.idx[channel]];
        self.idx[channel] += 1;
        v
    }
}

impl Clone for BufferedOgg {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            idx: [0, 0],
            channels: self.channels,
        }
    }
}

impl AudioNode for BufferedOgg {
    fn process(&mut self, _: &[Input], output: &mut [Buffer]) {
        for (ch, o) in output.iter_mut().enumerate() {
            for sample in o.iter_mut() {
                *sample = self.next(ch);
            }
        }
    }
}
