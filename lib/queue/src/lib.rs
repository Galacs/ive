extern crate redis;
use redis::{AsyncCommands, Commands, RedisError};

use std::thread::{self, JoinHandle};

use async_trait::async_trait;
use models::*;
use serde::{Deserialize, Serialize};

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

#[async_trait]
pub trait Queue {
    async fn send_job(&self, conn: &mut redis::aio::Connection) -> Result<u64, QueueError>;
    async fn receive_job(&self, conn: &mut redis::aio::Connection) -> Result<Job, QueueError>;
}

#[async_trait]
impl Queue for Job {
    async fn send_job(&self, conn: &mut redis::aio::Connection) -> Result<u64, QueueError> {
        match self {
            Job::EncodeToSize(video, _) => match video.as_ref().unwrap().url {
                VideoURI::Path(_) => {
                    let serialized = serde_json::to_string(self)?;
                    conn.lpush("queue", serialized).await?;
                    return Ok(conn.incr("nonce", 1).await?);
                }
                _ => return Ok(0),
            },
        }
    }
    async fn receive_job(&self, conn: &mut redis::aio::Connection) -> Result<Job, QueueError> {
        let str: String = conn.rpop("queue", None).await?;
        conn.decr("nonce", 1).await?;
        Ok(serde_json::from_str(&str)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let client = redis::Client::open("redis://192.168.0.58/").unwrap();
        let mut con = client.get_async_connection().await.unwrap();
        let job = Job::EncodeToSize(
            Some(Video {
                url: VideoURI::Path("".to_string()),
                id: "cmoil'id".to_owned(),
            }),
            EncodeToSizeParameters {
                target_size: 7 * 2_u32.pow(20),
            },
        );
        println!("{}", job.send_job(&mut con).await.unwrap());

        println!("{:?}", job.receive_job(&mut con).await.unwrap());

        assert_eq!(1, 4);
    }
}
