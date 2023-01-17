use std::process::Stdio;

use models::*;

extern crate ffmpeg_next as ffmpeg;

extern crate models;

pub mod encoding;
pub mod utils;

use tokio::process::{ChildStdout, Command};

pub async fn run_ffmpeg_upload(
    video: &Video,
    args: Option<Vec<&str>>,
    input_args: Option<Vec<&str>>,
    args_override: Option<Vec<&str>>,
) {
    let uri = &video.url;

    let url = match uri {
        VideoURI::Path(p) => p,
        VideoURI::Url(u) => u,
    };

    let a = match args_override {
        None => {
            let mut a = vec!["-y"];
            a.extend(args.unwrap());
            a.extend(["-i", url]);
            a.extend(input_args.unwrap());
            a.extend([
                "-f",
                "mp4",
                "-movflags",
                "frag_keyframe+empty_moov",
                "pipe:1",
            ]);
            a
        }
        Some(args) => args.iter().map(|x| *x).collect(),
    };

    let mut cmd = Command::new("ffmpeg");
    cmd.args(a);

    cmd.stdout(std::process::Stdio::piped());
    let mut child = cmd.spawn().expect("failed to spawn command");
    let mut stdout = child
        .stdout
        .take()
        .expect("child did not have a handle to stdout");

    let bucket = config::get_s3_bucket();
    let res = bucket
        .put_object_stream(&mut stdout, &video.id)
        .await
        .unwrap();
}

pub async fn remux(video: &Video, params: &RemuxParameters) -> Result<(), EncodeError> {
    let uri = &video.url;

    let url = match uri {
        VideoURI::Path(p) => p,
        VideoURI::Url(u) => u,
    };

    let format = match params.container {
        VideoContainer::MKV => "matroska",
        VideoContainer::MP4 => "mp4",
        VideoContainer::MP3 => todo!(),
        VideoContainer::WEBM => todo!(),
    };

    run_ffmpeg_upload(
        video,
        None,
        None,
        Some(vec![
            "-y",
            "-i",
            url,
            "-c",
            "copy",
            "-f",
            format,
            "-movflags",
            "frag_keyframe+empty_moov",
            "pipe:1",
        ]),
    )
    .await;
    Ok(())
}

pub async fn cut(video: &Video, params: &CutParameters) -> Result<(), EncodeError> {
    let mut args: Vec<&str> = Vec::new();

    let mut bf: Vec<&str> = Vec::new();

    let str;
    match &params.start {
        Some(time) => {
            str = time.to_string();
            bf.extend(vec!["-ss", &str]);
        }
        None => (),
    };

    let str;
    match &params.end {
        Some(time) => {
            str = time.to_string();
            bf.extend(vec!["-to", &str]);
        }
        None => (),
    };

    args.extend(vec!["-c:a", "copy", "-c:v", "copy"]);

    run_ffmpeg_upload(&video, Some(bf), Some(args), None).await;
    Ok(())
}

// Code without using lib

// pub fn encode_to_size(path: &str, t_size: f32, dest_path: &str) -> Result<(), EncodeToSizeError> {
//     if !Path::new(path).exists() {
//         return Err(EncodeToSizeError::new("file not found"));
//     }
//     // Get video duration
//     let mut duration_out = Command::new("ffprobe")
//         .args([
//             "-v",
//             "error",
//             "-show_entries",
//             "format=duration",
//             "-of",
//             "csv=p=0",
//             path,
//         ])
//         .output()
//         .unwrap();
//     duration_out.stdout.pop();
//     let duration = String::from_utf8(duration_out.stdout)
//         .unwrap()
//         .parse::<f32>()
//         .unwrap();
//     // println!("{:?}", duration);

//     // Get audio rate
//     let mut audio_rate_out = Command::new("ffprobe")
//         .args([
//             "-v",
//             "error",
//             "-select_streams",
//             "a:0",
//             "-show_entries",
//             "stream=bit_rate",
//             "-of",
//             "csv=p=0",
//             path,
//         ])
//         .output()
//         .unwrap();
//     audio_rate_out.stdout.pop();
//     let audio_rate_raw = String::from_utf8(audio_rate_out.stdout)
//         .unwrap()
//         .parse::<i32>()
//         .unwrap();
//     // println!("{:?}", audio_rate_raw);

//     // Original audio rate in KiB/s
//     let audio_rate = audio_rate_raw / 1024;

//     let t_minsize = (audio_rate as f32 * duration) / 8192_f32;
//     let size = t_size;

//     // Target size is required to be less than the size of the original audio stream
//     if t_minsize > size {
//         return Err(EncodeToSizeError::new("target size too small"));
//     }

//     let target_vrate = (size * 8192.0) / (1.048576 * duration) - audio_rate as f32;
//     // Perform the conversion
//     // 1st pass
//     let output = Command::new("ffmpeg")
//         .args([
//             "-y",
//             "-i",
//             path,
//             "-c:v",
//             "libx264",
//             "-b:v",
//             &format!("{}k", target_vrate),
//             "-pass",
//             "1",
//             "-an",
//             "-f",
//             "mp4",
//             "/dev/null",
//         ])
//         .output()
//         .unwrap();
//     // println!("{}", String::from_utf8(output.stdout).unwrap());

//     // 2nd pass
//     let output = Command::new("ffmpeg")
//         .args([
//             "-i",
//             path,
//             "-c:v",
//             "libx264",
//             "-b:v",
//             &format!("{}k", target_vrate),
//             "-pass",
//             "2",
//             "-c:a",
//             "aac",
//             "-b:a",
//             &format!("{}k", audio_rate),
//             dest_path,
//         ])
//         .output()
//         .unwrap();
//     // println!("{}", String::from_utf8(output.stdout).unwrap());

//     // Delete log files
//     for f in ["ffmpeg2pass-0.log", "ffmpeg2pass-0.log.mbtree"] {
//         if let Err(_) = fs::remove_file(f) {
//             return Err(EncodeToSizeError::new("can't delete log files"));
//         }
//     }
//     Ok(())
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let uri = VideoURI::Url("https://cdn.discordapp.com/attachments/685197521953488994/1048621810708648047/clip-00.18.52.873-00.19.07.444-8MB.mp4".to_owned());

        let video = Video::new(
            uri,
            Some("dfkgjsdpfmkgj.mp4".to_owned()),
            "toz123".to_owned(),
        );

        cut(
            &video,
            &CutParameters {
                start: Some(2),
                end: None,
            },
        )
        .await
        .unwrap();
        assert_ne!(0, 0);
    }
}
