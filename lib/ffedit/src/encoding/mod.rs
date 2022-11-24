use std::{path::Path, process::Command};

use crate::utils;

use models::*;

#[derive(Debug)]
pub struct EncodeToSizeError {
    pub details: String,
}

impl EncodeToSizeError {
    fn new(msg: &str) -> EncodeToSizeError {
        EncodeToSizeError {
            details: msg.to_string(),
        }
    }
}

pub fn encode_to_size(video: &Video, params: EncodeToSizeParameters, dest_path: &str) -> Result<(), EncodeToSizeError> {

    let path;

    match &video.url {
        VideoURI::Path(p) => {path = Path::new(p)},
        _ => return Ok(()),
    }

    if !path.exists() {
        return Err(EncodeToSizeError::new("file not found"));
    }

    ffmpeg::init().unwrap();

    let input = ffmpeg::format::input(&path).unwrap();

    let duration = utils::get_duration(&input);
    // println!("duration :{}s", duration);

    let audio_rate = utils::get_audio_bitrate(&input);
    // println!("audio bitrate: {} kbit/s", &audio_rate);

    let t_minsize = (audio_rate as f32 * duration) / 8192_f32;
    let size: f32 = params.target_size as f32 / 2_f32.powf(20.0);
    if t_minsize > size {
        return Err(EncodeToSizeError::new("target size too small"));
    }

    let target_vrate = (size * 8192.0) / (1.048576 * duration) - audio_rate as f32;
    // println!("target video bitrate: {}kbit/s", target_vrate);

    let mut dir = path.to_path_buf();
    dir.pop();

    // 1st pass
    let output = Command::new("ffmpeg")
        .current_dir(&dir)
        .args([
            "-y",
            "-i",
            path.to_str().unwrap(),
            "-c:v",
            "libx264",
            "-b:v",
            &format!("{}k", target_vrate),
            "-pass",
            "1",
            "-an",
            "-f",
            "mp4",
            "/dev/null",
        ])
        .output()
        .unwrap();

    // 2nd pass
    let output = Command::new("ffmpeg")
        .current_dir(&dir)
        .args([
            "-i",
            path.to_str().unwrap(),
            "-c:v",
            "libx264",
            "-b:v",
            &format!("{}k", target_vrate),
            "-pass",
            "2",
            "-c:a",
            "aac",
            "-b:a",
            &format!("{}k", audio_rate),
            dest_path,
        ])
        .output()
        .unwrap();

    Ok(())
}
