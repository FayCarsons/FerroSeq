use std::{f32::consts::TAU, sync::mpsc::Receiver};

use super::destruction;

const DEFAULT_SAMPLE_RATE: usize = 48_000;
const DEFAULT_SLICES: usize = 16;

const DISTORTION_PARAMS: destruction::Params = destruction::Params {
    pregain: 4.,
    postgain: 1.,
    bit_depth: 32,
    downsample_factor: 2,
    resolution: 32.,
    noise_amount: 0.1,
    feedback: 0.1,
};

// I'm going to need this for retrigger
#[allow(unused)]
fn hanning(phase: f32, size: usize) -> f32 {
    let x = (TAU * phase) / size as f32;
    0.5 * (1. - x.cos())
}

fn lerp(fst: f32, snd: f32, t: f32) -> f32 {
    fst * (1. - t) + snd * t
}

#[derive(Debug, Clone, Copy)]
pub struct StepBuilder {
    slice: usize,
    pitch: f32,
    direction: Direction,
}

impl Default for StepBuilder {
    fn default() -> Self {
        Self {
            slice: 0,
            pitch: 1.,
            direction: Direction::Forward,
        }
    }
}

impl StepBuilder {
    pub fn slice(&self) -> usize {
        self.slice
    }

    pub fn with_slice(self, slice: usize) -> Self {
        Self { slice, ..self }
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    pub fn with_pitch(self, pitch: f32) -> Self {
        Self { pitch, ..self }
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn with_direction(self, direction: Direction) -> Self {
        Self { direction, ..self }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Step {
    On(StepBuilder),
    Off,
}

impl Default for Step {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Forward,
    Backward,
}

pub struct Sampler {
    samples: Vec<f32>,
    slice_len: usize,
    sample_rate: usize,
    playing: bool,
    pos: f32,
    current_slice: usize,
    speed: f32,
    start: f32,
    end: f32,
    direction: Direction,
    distortion: destruction::Destruction,
    channel: Receiver<Step>,
}

fn wrap<T>(n: T, lo: T, hi: T) -> T
where
    T: PartialOrd,
{
    if n > hi {
        lo
    } else if n < lo {
        hi
    } else {
        n
    }
}

impl Sampler {
    pub fn new(samples: Vec<f32>, channel: Receiver<Step>) -> Self {
        let len = samples.len();
        let slice_len = len / DEFAULT_SLICES;
        let end = len as f32;
        Self {
            samples,
            slice_len,
            sample_rate: DEFAULT_SAMPLE_RATE,
            current_slice: 0,
            pos: 0.,
            playing: false,
            speed: 1.,
            start: 0.,
            end,
            distortion: destruction::Destruction::default(),
            direction: Direction::Forward,
            channel,
        }
    }

    fn interpolate(&self) -> f32 {
        let fst = wrap(
            self.pos.floor() as usize,
            self.start as usize,
            self.end as usize - 1,
        );
        let snd = match self.direction {
            Direction::Forward => wrap(fst + 1, self.start as usize, self.end as usize - 1),
            Direction::Backward => wrap(fst - 1, self.start as usize, self.end as usize - 1),
        };
        let frac = self.pos.fract();
        lerp(self.samples[fst], self.samples[snd], frac).tanh()
    }

    fn handle_message(&mut self) {
        let mut count = 4;
        while let Ok(step) = self.channel.try_recv() {
            match step {
                Step::On(StepBuilder {
                    slice,
                    pitch,
                    direction,
                }) => {
                    self.current_slice = slice;
                    self.pos = match direction {
                        Direction::Forward => (slice * self.slice_len) as f32,
                        Direction::Backward => (slice * self.slice_len + self.slice_len - 1) as f32,
                    };
                    self.speed = pitch;

                    self.direction = direction;
                    self.playing = true;
                }

                Step::Off => self.playing = false,
            }

            count -= 1;
            if count == 0 {
                break;
            }
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.distortion.set_sample_rate(sample_rate)
    }

    fn advance(&mut self) {
        match self.direction {
            Direction::Forward => {
                self.pos += self.speed;
            }
            Direction::Backward => {
                self.pos -= self.speed;
            }
        }

        self.pos = wrap(self.pos, self.start, self.end - 1.);
    }

    fn slice_ended(&mut self) {
        match self.direction {
            Direction::Forward => {
                let slice_start = self.slice_len * self.current_slice;
                let slice_end = slice_start + self.slice_len;
                let pos = self.pos as usize;
                self.playing = pos >= slice_start && pos < slice_end;
            }
            Direction::Backward => {
                let slice_start = self.slice_len * self.current_slice;
                let slice_end = slice_start + self.slice_len;
                let pos = self.pos as usize;
                self.playing = pos <= slice_end && pos > slice_start;
            }
        }
    }

    fn process_effects(&mut self, sample: f32) -> f32 {
        destruction::Destruction::tick(&mut self.distortion, sample, DISTORTION_PARAMS)
    }

    pub fn tick(&mut self) -> f32 {
        self.handle_message();

        if self.playing {
            self.advance();
            let sample = self.interpolate();
            self.slice_ended();
            self.process_effects(sample)
        } else {
            0.
        }
    }
}
