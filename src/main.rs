mod app;
mod common;
mod decode;
mod destruction;
mod metro;
mod sampler;
mod stream;
mod widgets;
use app::App;
use cpal::traits::StreamTrait;
use sampler::{Sampler, Step};
use std::path::Path;

fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    let path = Path::new("amen.wav");
    let (samples, _frames_collected) = decode::decode(path);
    let total_len = samples.len();
    println!("Got {} samples", total_len);
    let (sender, receiver) = std::sync::mpsc::channel::<Step>();
    let sample_player = Sampler::new(samples, receiver);

    let stream = stream::setup(sample_player).unwrap();
    stream.play().unwrap();
    match monome::Monome::enumerate_devices().as_deref() {
        Ok([grid]) => match App::new(grid, sender) {
            Ok(state) => state.run(),
            Err(e) => {
                println!("Setup failed: {e}");
            }
        },
        Ok(_) => println!("Grid not found :3"),
        Err(e) => panic!("Monome error: {e}"),
    }

    Ok(())
}
