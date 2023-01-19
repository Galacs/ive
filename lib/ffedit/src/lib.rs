use std::{process::Stdio, path::{PathBuf, Path}};

use models::*;

extern crate ffmpeg_next as ffmpeg;

pub mod utils;

use ffmpeg_cli::{FfmpegBuilder, File, Parameter};

use async_trait::async_trait;

#[async_trait]
pub trait Run {
    async fn run_and_upload(self, id: &str);
}

#[async_trait]
impl Run for FfmpegBuilder<'_> {
    async fn run_and_upload(self, id: &str) {
        let ffmpeg = self.run().await;

        let child = ffmpeg.unwrap().process;
        let mut stdout = child.stdout.unwrap();
        // let mut stderr = child.stderr.unwrap();

        let bucket = config::get_s3_bucket();
        let _res = bucket
            .put_object_stream(&mut stdout, &id)
            .await
            .unwrap();
    }
}
pub trait FfmpegBuilderDefault<'a> {
    fn default(url: &str) -> FfmpegBuilder;
}

impl<'a> FfmpegBuilderDefault<'a> for FfmpegBuilder<'a> {
    fn default(url: &str) -> FfmpegBuilder {
        FfmpegBuilder {
            options: vec![Parameter::Single("nostdin"), Parameter::Single("y")],
            inputs: vec![File::new(url)],
            outputs: vec![File::new("pipe:1").option(Parameter::KeyValue("f", "mp4")).option(Parameter::KeyValue("movflags", "frag_keyframe+empty_moov"))],
            ffmpeg_command: "ffmpeg",
            stdin: Stdio::null(),
            stdout: Stdio::piped(),
            stderr: Stdio::inherit(),
        }
    }

}

pub fn get_working_dir(id: &String) -> Result<PathBuf, InteractionError> {
    let dir = Path::new("tmpfs/").join(format!("{}", id));
    let dir = std::env::current_dir()?.join(dir);
    Ok(dir)
}

pub async fn encode_to_size(video: &Video, params: &EncodeToSizeParameters) -> Result<(), EncodeError> {
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
    // dbg!(&dir);
    // tokio::fs::create_dir(&dir).await.unwrap();

    let mut builder = FfmpegBuilder::default(url);
    builder.stdout = Stdio::null();

    let target_vrate = format!("{}k", target_vrate);
    let audio_rate = format!("{}k", audio_rate);

    let a = dir.join(Path::new("pass"));
    let passfile_prefix = a.to_str().unwrap();

    let file = File::new("pipe:1").option(Parameter::KeyValue("f", "mp4"))
    .option(Parameter::KeyValue("movflags", "frag_keyframe+empty_moov"))
    .option(Parameter::KeyValue("c:v", "libx264"))
    .option(Parameter::KeyValue("b:v", &target_vrate))
    .option(Parameter::KeyValue("pass", "1"))
    .option(Parameter::Single("an"))
    .option(Parameter::KeyValue("passlogfile", passfile_prefix));
    builder.outputs = vec![file];

    builder.run().await.unwrap().process.wait().await.unwrap();
 
    let mut builder = FfmpegBuilder::default(url);

    let file = File::new("pipe:1").option(Parameter::KeyValue("f", "mp4"))
    .option(Parameter::KeyValue("movflags", "frag_keyframe+empty_moov"))
    .option(Parameter::KeyValue("c:v", "libx264"))
    .option(Parameter::KeyValue("b:v", &target_vrate))
    .option(Parameter::KeyValue("pass", "2"))
    .option(Parameter::KeyValue("c:a", "aac"))
    .option(Parameter::KeyValue("b:a", &audio_rate))
    .option(Parameter::KeyValue("passlogfile", passfile_prefix));
    builder.outputs = vec![file];

    builder.run_and_upload(&video.id).await;
    Ok(())
}

pub async fn remux(video: &Video, params: &RemuxParameters) -> Result<(), EncodeError> {
    let url = match &video.url {
        VideoURI::Path(p) => p,
        VideoURI::Url(u) => u,
    };

    let format = match params.container {
        VideoContainer::MKV => "matroska",
        VideoContainer::MP4 => "mp4",
        VideoContainer::MP3 => todo!(),
        VideoContainer::WEBM => todo!(),
    };

    let mut builder = FfmpegBuilder::default(url);
    let file = File::new("pipe:1").option(Parameter::KeyValue("f", format)).option(Parameter::KeyValue("movflags", "frag_keyframe+empty_moov"));
    builder.outputs = vec![file];

    builder.run_and_upload(&video.id).await;
    Ok(())
}

pub async fn cut(video: &Video, params: &CutParameters) -> Result<(), EncodeError> {
    let url = match &video.url {
        VideoURI::Path(u) => u,
        VideoURI::Url(u) => u,
    };
    let mut builder = FfmpegBuilder::default(url);
    
    let str;
    if let Some(time) = params.start {
        str = time.to_string();
        builder = builder.option(Parameter::KeyValue("ss", &str));
    }
    let str;
    if let Some(time) = params.end {
        str = time.to_string();
        builder = builder.option(Parameter::KeyValue("to", &str));
    }

    builder.run_and_upload(&video.id).await;
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
