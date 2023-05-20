use s3::error::S3Error;
use serde::{Deserialize, Serialize};
use serenity::prelude::SerenityError;
use snafu::Snafu;
use thiserror::Error;

#[derive(Error, Serialize, Deserialize, Debug)]
pub enum EncodeToSize {
    #[error("Unsupported URL")]
    UnsupportedURI,
    #[error("Target size too small")]
    TargetSizeTooSmall,
}

#[derive(Error, Serialize, Deserialize, Debug)]
#[error("Encode error: {0}")]
pub enum Encode {
    EncodeToSize(EncodeToSize),
}


use redis::RedisError;

#[derive(Debug)]
pub enum Queue {
    Redis(RedisError),
    Serde(serde_json::Error),
}

impl From<RedisError> for Queue {
    fn from(error: RedisError) -> Self {
        Queue::Redis(error)
    }
}

impl From<serde_json::Error> for Queue {
    fn from(error: serde_json::Error) -> Self {
        Queue::Serde(error)
    }
}


#[derive(Debug)]
pub enum Edit {
    WrongFileNumber(u32),
}

#[derive(Error, Debug)]
pub enum InvalidInput {
    #[error("Invalid input error")]
    Error,
    #[error("Invalid parse float Error: {0:?}")]
    StringParse(#[from] std::num::ParseFloatError),
}

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
pub enum Ffmpeg {
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
pub enum Worker {
    InvalidInput {
        source: InvalidInput,
        backtrace: snafu::Backtrace,
        location: snafu::Location,
    },
    Ffmpeg {
        source: Ffmpeg,
        backtrace: snafu::Backtrace,
        location: snafu::Location,
    },
    Encode {
        source: Encode,
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
pub enum Interaction {
    Queue(Queue),
    Serenity(SerenityError),
    Error,
    NotImplemented,
    Edit(Edit),
    Timeout,
    Io(std::io::Error),
    InvalidInput(InvalidInput),
    Redis(redis::RedisError),
    S3(S3Error),
    Serde(serde_json::Error),
}

impl From<SerenityError> for Interaction {
    fn from(error: SerenityError) -> Self {
        Interaction::Serenity(error)
    }
}

impl From<std::io::Error> for Interaction {
    fn from(error: std::io::Error) -> Self {
        Interaction::Io(error)
    }
}

impl From<std::num::ParseFloatError> for Interaction {
    fn from(error: std::num::ParseFloatError) -> Self {
        Interaction::InvalidInput(InvalidInput::StringParse(error))
    }
}

impl From<redis::RedisError> for Interaction {
    fn from(error: redis::RedisError) -> Self {
        Interaction::Redis(error)
    }
}

impl From<S3Error> for Interaction {
    fn from(error: S3Error) -> Self {
        Interaction::S3(error)
    }
}

impl From<serde_json::Error> for Interaction {
    fn from(error: serde_json::Error) -> Self {
        Interaction::Serde(error)
    }
}

impl From<Queue> for Interaction {
    fn from(error: Queue) -> Self {
        Interaction::Queue(error)
    }
}