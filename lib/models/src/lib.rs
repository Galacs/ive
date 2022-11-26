use serde::{Deserialize, Serialize};
use serenity::prelude::SerenityError;

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
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Job {
    EncodeToSize(Option<Video>, EncodeToSizeParameters),
}

impl Video {
    pub fn new(uri: VideoURI, id: Option<String>) -> Video {
        if let Some(str) = id {
            return Video { url: uri, id: str };
        }
        Video {
            url: uri,
            id: "".to_owned(),
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
        fn from(errror: serde_json::Error) -> Self {
            QueueError::Serde(errror)
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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
