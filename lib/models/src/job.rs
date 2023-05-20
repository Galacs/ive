use serde::{Serialize, Deserialize};
use crate::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    GetStreams(Vec::<MediaStream>)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Progress {
    Started,
    Progress(f32),
    Error(String),
    Response(job::Response),
    Done(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Kind {
    Processing,
    Parsing,
}

#[derive(Serialize, Deserialize, Debug)]

pub enum Parameters {
    EncodeToSize(EncodeToSizeParameters),
    Cut(CutParameters),
    Remux(RemuxParameters),
    GetStreams,
    Combine(CombineParameters),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Job {
    pub kind: Kind,
    pub video: Option<Video>,
    pub params: Parameters,

}

impl Job {
    pub fn new(kind: Kind, video: Option<Video>, params: Parameters) -> Self {
        Job { kind, video, params }
    }
}