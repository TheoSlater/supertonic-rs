use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::Path;

pub fn write_wav_file<P: AsRef<Path>>(
    filename: P,
    audio_data: &[f32],
    sample_rate: u32,
) -> Result<(), anyhow::Error> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(filename, spec)?;

    for &sample in audio_data {
        let clamped = sample.max(-1.0).min(1.0);
        let val = (clamped * 32767.0) as i16;
        writer.write_sample(val)?;
    }

    writer.finalize()?;
    Ok(())
}

pub fn encode_wav_bytes(audio_data: &[f32], sample_rate: u32) -> Result<Vec<u8>, anyhow::Error> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::new(&mut cursor, spec)?;
    for &sample in audio_data {
        let clamped = sample.max(-1.0).min(1.0);
        let val = (clamped * 32767.0) as i16;
        writer.write_sample(val)?;
    }
    writer.finalize()?;
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_wav_bytes_returns_valid_header() {
        let audio = vec![0.0f32; 44100];
        let bytes = encode_wav_bytes(&audio, 44100).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert!(!bytes.is_empty());
    }
}
