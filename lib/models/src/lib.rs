use s3::error::S3Error;
use serde::{Deserialize, Serialize};
use serenity::prelude::SerenityError;
use snafu::Snafu;
use thiserror::Error;

pub mod job;
pub mod error;

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodeToSizeParameters {
    pub target_size: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CutParameters {
    pub start: Option<u32>,
    pub end: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RemuxParameters {
    pub container: VideoContainer,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CombineVideo {
    pub url: String,
    pub selected_streams: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CombineParameters {
    pub videos: Vec<CombineVideo>,
    pub output_kind: StreamKind,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum VideoContainer {
    MP3, 
    MP4,
    WEBM,
    MKV,
}


#[derive(Serialize, Deserialize, Debug)]
pub enum VideoURI {
    Path(String),
    Url(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Video {
    pub url: VideoURI,
    pub id: String,
    pub filename: String,
}

impl Video {
    pub fn new(url: VideoURI, id: Option<String>, filename: String) -> Video {
        if let Some(str) = id {
            return Video { url, id: str, filename };
        }
        Video {
            url,
            id: "".to_owned(),
            filename
        }
    }
}

impl VideoContainer {
    pub fn get_file_extension(&self) -> String {
        match self {
            VideoContainer::MKV => "mkv",
            VideoContainer::MP4 => "mp4",
            VideoContainer::MP3 => "mp3",
            VideoContainer::WEBM => "webm",
        }.to_owned()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum StreamKind {
    Video,
    Audio,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MediaStream {
    pub id: usize,
    pub kind: StreamKind,
    pub duration: i64,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() { }
}
