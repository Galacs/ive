use ffmpeg::format::context::Input;


#[inline(always)]
pub fn get_duration(input: &Input) -> f32 {
    input.duration() as f32 / 10.0_f32.powf(6.0)
}

#[inline(always)]
pub fn get_audio_bitrate(input: &Input) -> f32 {
    for stream in input.streams() {
        let codec = ffmpeg::codec::context::Context::from_parameters(stream.parameters()).unwrap();
        if codec.medium() == ffmpeg::media::Type::Audio {
            if let Ok(audio) = codec.decoder().audio() {
                return audio.bit_rate() as f32 / 1024.0;
            }
        }
    }
    0.0
}