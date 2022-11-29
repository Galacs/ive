use s3::error::S3Error;
use serde::{Deserialize, Serialize};
use serenity::prelude::SerenityError;


#[derive(Serialize, Deserialize, Debug)]
pub enum EncodeToSizeError {
    UnsupportedURI,
    TargetSizeTooSmall,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum EncodeError {
    EncodeToSize(EncodeToSizeError),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum JobProgress {
    Started,
    Progress(f32),
    Error(EncodeError),
    Done,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodeToSizeParameters {
    pub target_size: u32,
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

#[derive(Serialize, Deserialize, Debug)]
pub enum JobKind {
    EncodeToSize,
}

#[derive(Serialize, Deserialize, Debug)]

pub enum EncodeParameters {
    EncodeToSize(EncodeToSizeParameters)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Job {
    pub kind: JobKind,
    pub video: Option<Video>,
    pub params: EncodeParameters,
}

impl Job {
    pub fn new(kind: JobKind, video: Option<Video>, params: EncodeParameters) -> Self {
        Job { kind, video, params }
    }
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

#[derive(Debug)]
pub enum InvalidInputError {
    Error,
    StringParse(std::num::ParseFloatError),
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
