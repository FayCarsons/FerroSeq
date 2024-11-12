use super::sampler::Sampler;
use cpal::traits::{DeviceTrait, HostTrait};

pub fn setup(mut sample_player: Sampler) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("Cannot get default device");
    let config = device.default_output_config().map_err(|e| e.to_string())?;
    sample_player.set_sample_rate(config.sample_rate().0 as usize);

    match config.sample_format() {
        cpal::SampleFormat::F32 => make_stream(sample_player, device, &config.config()),
        _ => Err("Cannot get f32 sample format".to_string()),
    }
}

fn make_stream(
    sample_player: Sampler,
    device: cpal::Device,
    config: &cpal::StreamConfig,
) -> Result<cpal::Stream, String> {
    let on_error = |e| eprintln!("Error in audio thread: {e}");

    let stream = device
        .build_output_stream(config, create_update_fn(sample_player), on_error, None)
        .expect("Cannot build stream");

    Ok(stream)
}

fn create_update_fn(
    mut sample_player: Sampler,
) -> impl FnMut(&mut [f32], &cpal::OutputCallbackInfo) {
    move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
        for frame in output.chunks_mut(2) {
            let sample = sample_player.tick();

            frame.fill(sample);
        }
    }
}
