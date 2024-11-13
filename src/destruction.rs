/// A 90s Zoom multi-effects/NIN inspired digital distortion

#[derive(Debug, Clone)]
pub struct Destruction {
    sample_rate: f32,
    downsample_count: usize,
    prev_sample: f32,
    noise_phase: f32,
}

impl Default for Destruction {
    fn default() -> Self {
        Self {
            sample_rate: 44_800f32,
            downsample_count: 0,
            prev_sample: 0f32,
            noise_phase: 0f32,
        }
    }
}

pub struct Params {
    pub pregain: f32,
    pub postgain: f32,
    pub bit_depth: usize,
    pub downsample_factor: usize,
    pub resolution: f32,
    pub noise_amount: f32,
    pub feedback: f32,
}

impl Params {
    // Suggested "NIN inspired" preset
    pub const fn nin() -> Self {
        Self {
            pregain: 8.,
            postgain: 0.6,
            bit_depth: 8,
            downsample_factor: 6,
            resolution: 16.,
            noise_amount: 0.04,
            feedback: 0.3,
        }
    }
}

impl Destruction {
    pub fn set_sample_rate(&mut self, sample_rate: usize) {
        self.sample_rate = sample_rate as f32;
    }

    pub fn tick(&mut self, input: f32, params: Params) -> f32 {
        let mut signal = input * params.pregain;

        // Downsample the signal - only sample the input every `params.downsample_factor` ticks
        signal = if self.downsample_count == 0 {
            let quantize_steps = (1 << params.bit_depth) as f32;
            (signal * quantize_steps).round() / quantize_steps
        } else {
            self.prev_sample
        };

        self.downsample_count = (self.downsample_count + 1) % params.downsample_factor;
        self.prev_sample = signal;

        // Add asymmetry
        signal = signal.clamp(-0.98, 1.);

        self.noise_phase = (self.noise_phase
            + 0.1 * (2.0 * std::f32::consts::PI / self.sample_rate))
            % std::f32::consts::TAU;
        let noise = self.noise_phase.sin() * params.noise_amount;
        signal += noise;

        let step_amount = 1. / params.resolution;
        signal = (signal / step_amount).round() * step_amount;

        signal += self.prev_sample * params.feedback;

        signal *= params.postgain;
        signal.clamp(-1., 1.)
    }
}
