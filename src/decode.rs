use std::io::ErrorKind;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Returns a tuple of samples and number of frames collected
pub fn decode(path: &Path) -> (Vec<f32>, u64) {
    let src = std::fs::File::open(path).expect("Cannot find amen break");

    let mstream = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|os| os.to_str()) {
        hint.with_extension(extension);
    }

    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mstream, &fmt_opts, &meta_opts)
        .expect("Unsupported format");

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .expect("No supported audio tracks");

    let expected_frames = track.codec_params.n_frames;
    let num_channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(1);
    // Print track details
    println!("Track info:");
    println!("  Codec: {:?}", track.codec_params.codec);
    println!("  Sample Rate: {:?}", track.codec_params.sample_rate);
    println!("  Channels: {:?}", track.codec_params.channels);
    if let Some(n_frames) = track.codec_params.n_frames {
        println!("  Expected frames: {}", n_frames);
    }

    let dec_opts: DecoderOptions = Default::default();

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .expect("Unsupported codec");

    let mut samples_interleaved = Vec::<f32>::new();
    let mut frames_collected = 0;
    loop {
        match format.next_packet().and_then(|p| decoder.decode(&p)) {
            Ok(decoded) => {
                let spec = decoded.spec();
                let frames = decoded.frames() as u64;
                frames_collected += frames;

                let duration = decoded.capacity() as u64;

                let mut sample_buf = SampleBuffer::new(duration, *spec);
                sample_buf.copy_interleaved_ref(decoded);
                samples_interleaved.extend(sample_buf.samples());
            }
            Err(Error::IoError(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                println!("Hit EOF");
                break;
            }
            Err(e) => panic!("Decode error: {e}"),
        }
    }

    if let Some(ef) = expected_frames {
        assert_eq!(ef, frames_collected);
        println!("OK: Frames collected == expected_frames")
    }

    let samples = samples_interleaved
        .chunks(num_channels)
        .map(|frame| frame.iter().sum())
        .collect();

    (samples, frames_collected)
}
