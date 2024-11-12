use std::{f32::consts::PI, sync::mpsc::Receiver};

const DEFAULT_SAMPLE_RATE: usize = 48_000;
const DEFAULT_SLICES: usize = 16;

pub trait Window {
    fn window(phase: f32, size: usize) -> f32;
}

pub struct Hanning;

impl Window for Hanning {
    fn window(phase: f32, size: usize) -> f32 {
        let x = (2. * PI * phase) / size as f32;
        0.5 * (1. - x.cos())
    }
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

impl Step {
    pub fn map<F>(self, f: F) -> Self
    where
        F: Fn(StepBuilder) -> StepBuilder,
    {
        match self {
            Self::On(step) => Self::On(f(step)),
            Self::Off => Self::Off,
        }
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
    channel: Receiver<Step>,
}

fn wrapf(n: f32, lo: f32, hi: f32) -> f32 {
    if n >= hi {
        lo
    } else if n < lo {
        hi
    } else {
        n
    }
}

fn wrapu(n: usize, lo: usize, hi: usize) -> usize {
    if n >= hi {
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
            direction: Direction::Forward,
            channel,
        }
    }

    fn interpolate(&self) -> f32 {
        let fst = wrapu(
            self.pos.floor() as usize,
            self.start as usize,
            self.end as usize,
        );
        let snd = match self.direction {
            Direction::Forward => wrapu(fst + 1, self.start as usize, self.end as usize),
            Direction::Backward => wrapu(fst - 1, self.start as usize, self.end as usize),
        };
        let frac = self.pos.fract();
        lerp(self.samples[fst], self.samples[snd], frac).tanh()
    }

    fn wrap_playhead(&mut self) {
        if self.pos >= self.end {
            self.pos = self.start
        } else if self.pos < self.start {
            self.pos = self.end - 1.
        }
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
        self.sample_rate = sample_rate
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

        self.wrap_playhead();
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

    pub fn tick(&mut self) -> f32 {
        self.handle_message();

        if self.playing {
            self.advance();
            let sample = self.interpolate();
            self.slice_ended();
            sample
        } else {
            0.
        }
    }
}
