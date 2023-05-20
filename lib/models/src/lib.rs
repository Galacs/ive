use s3::error::S3Error;
use serde::{Deserialize, Serialize};
use serenity::prelude::SerenityError;
use snafu::Snafu;
use thiserror::Error;

pub mod job;

#[derive(Error, Serialize, Deserialize, Debug)]
pub enum EncodeToSizeError {
    #[error("Unsupported URL")]
    UnsupportedURI,
    #[error("Target size too small")]
    TargetSizeTooSmall,
}

#[derive(Error, Serialize, Deserialize, Debug)]
#[error("Encode error: {0}")]
pub enum EncodeError {
    EncodeToSize(EncodeToSizeError),
}

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

pub mod queue {
    use redis::RedisError;

    #[derive(Debug)]
    pub enum QueueError {
        Redis(RedisError),
        Serde(serde_json::Error),
    }

    impl From<RedisError> for QueueError {
        fn from(error: RedisError) -> Self {
            QueueError::Redis(error)
        }
    }

    impl From<serde_json::Error> for QueueError {
        fn from(error: serde_json::Error) -> Self {
            QueueError::Serde(error)
        }
    }
}

#[derive(Debug)]
pub enum EditError {
    WrongFileNumber(u32),
}

#[derive(Error, Debug)]
pub enum InvalidInputError {
    #[error("Invalid input error")]
    Error,
    #[error("Invalid parse float Error: {0:?}")]
    StringParse(#[from] std::num::ParseFloatError),
}

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum FfmpegError {
    FfIo {
        source: std::io::Error,
    },
    KeyValueParse {
        key: String,
    },
    UnknownStatus {
        status: String,
    },
    OtherParse {
        source: Box<dyn std::error::Error + Send>,
        msg: String,
    },
    Exit{
        status: std::process::ExitStatus,
        location: snafu::Location,
    },
}

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum WorkerError {
    InvalidInput {
        source: InvalidInputError,
        backtrace: snafu::Backtrace,
        location: snafu::Location,
    },
    Ffmpeg {
        source: FfmpegError,
        backtrace: snafu::Backtrace,
        location: snafu::Location,
    },
    Encode {
        source: EncodeError,
        backtrace: snafu::Backtrace,
        location: snafu::Location,
    },
    Io {
        source: std::io::Error,
        backtrace: snafu::Backtrace,
        location: snafu::Location,
    },
    Message {
        msg: String,
    },
    S3 {
        source:  S3Error,
        backtrace: snafu::Backtrace,
        location: snafu::Location,
    },
}

#[derive(Debug)]
pub enum InteractionError {
    Queue(queue::QueueError),
    Serenity(SerenityError),
    Error,
    NotImplemented,
    Edit(EditError),
    Timeout,
    Io(std::io::Error),
    InvalidInput(InvalidInputError),
    Redis(redis::RedisError),
    S3(S3Error),
    Serde(serde_json::Error),
}

impl From<SerenityError> for InteractionError {
    fn from(error: SerenityError) -> Self {
        InteractionError::Serenity(error)
    }
}

impl From<std::io::Error> for InteractionError {
    fn from(error: std::io::Error) -> Self {
        InteractionError::Io(error)
    }
}

impl From<std::num::ParseFloatError> for InteractionError {
    fn from(error: std::num::ParseFloatError) -> Self {
        InteractionError::InvalidInput(InvalidInputError::StringParse(error))
    }
}

impl From<redis::RedisError> for InteractionError {
    fn from(error: redis::RedisError) -> Self {
        InteractionError::Redis(error)
    }
}

impl From<S3Error> for InteractionError {
    fn from(error: S3Error) -> Self {
        InteractionError::S3(error)
    }
}

impl From<serde_json::Error> for InteractionError {
    fn from(error: serde_json::Error) -> Self {
        InteractionError::Serde(error)
    }
}

impl From<queue::QueueError> for InteractionError {
    fn from(error: queue::QueueError) -> Self {
        InteractionError::Queue(error)
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
