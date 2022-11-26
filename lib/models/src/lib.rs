use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodeToSizeParameters {
    pub target_size: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum VideoURI {
    Path(String),
    Url(String)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Video {
    pub url: VideoURI,
    pub id: String
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Job {
    EncodeToSize(Option<Video>, EncodeToSizeParameters)
}

impl Video {
    pub fn new(uri: VideoURI, id: Option<String>) -> Video {
        if let Some(str) = id {
            return Video { url: uri, id: str }
        }
        Video { url: uri, id: "".to_owned() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {

    }
}
