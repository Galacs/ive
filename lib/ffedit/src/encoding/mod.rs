use std::{path::{Path, PathBuf}, process::Command, io::Write};

use crate::utils;

use models::*;
use s3::creds::Credentials;
use tokio::{io::{AsyncWriteExt, AsyncReadExt}, process::ChildStdout};

pub fn get_working_dir(id: &String) -> Result<PathBuf, InteractionError> {
    let dir = Path::new("tmpfs/").join(format!("{}", id));
    let dir = std::env::current_dir()?.join(dir);
    Ok(dir)
}

pub async fn encode_to_size(video: &Video, params: &EncodeToSizeParameters) -> Result<(), EncodeError> {

    use tokio::process::Command;

    let url = match &video.url {
        VideoURI::Url(p) => p,
        _ => return Err(EncodeError::EncodeToSize(EncodeToSizeError::UnsupportedURI)),
    };

    ffmpeg::init().unwrap();

    let input = ffmpeg::format::input(url).unwrap();

    let duration = utils::get_duration(&input);

    let audio_rate = utils::get_audio_bitrate(&input);

    let t_minsize = (audio_rate as f32 * duration) / 8192_f32;
    let size: f32 = params.target_size as f32 / 2_f32.powf(20.0);
    if t_minsize > size {
        return Err(EncodeError::EncodeToSize(EncodeToSizeError::TargetSizeTooSmall));
    }

    let target_vrate = (size * 8192.0) / (1.048576 * duration) - audio_rate as f32;

    let dir = get_working_dir(&video.id).unwrap();

    // 1st pass
    let output = Command::new("ffmpeg").current_dir(&dir).args([
        "-y",
        "-i",
        url,
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
    ]).spawn().unwrap().wait().await.unwrap();


    // 2nd pass
    let mut cmd = Command::new("ffmpeg");
    cmd.current_dir(&dir)
        .args([
            "-i",
            url,
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
            "-f",
            "mp4",
            "-movflags",
            "frag_keyframe+empty_moov",
            "pipe:1",
        ]);


    cmd.stdout(std::process::Stdio::piped());

    let mut child = cmd.spawn()
        .expect("failed to spawn command");

    let mut stdout = child.stdout.take()
        .expect("child did not have a handle to stdout");


    let bucket = config::get_s3_bucket();
    let res = bucket.put_object_stream(&mut stdout, video.id.to_owned()).await.unwrap();
    
    let status = child.wait().await.unwrap();
    
    Ok(())
}
