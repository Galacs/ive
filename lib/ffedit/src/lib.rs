extern crate ffmpeg_next as ffmpeg;

extern crate models;

pub mod utils;
pub mod encoding;


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
    use std::path::Path;

    use super::*;

    #[test]
    fn it_works() {
        // let _ = encoding::encode_to_size(Path::new("in.mp4"), 8.0, "out.mp4");
        assert_ne!(0, 0);
    }
}
