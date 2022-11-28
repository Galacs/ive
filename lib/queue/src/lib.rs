extern crate redis;
use redis::AsyncCommands;

use async_trait::async_trait;
use models::queue::QueueError;
use models::*;

#[async_trait]
pub trait Queue {
    async fn send_job(&self, conn: &mut redis::aio::Connection) -> Result<u64, QueueError>;
    async fn receive_job(conn: &mut redis::aio::Connection) -> Result<Job, QueueError>;
}

#[async_trait]
impl Queue for Job {
    async fn send_job(&self, conn: &mut redis::aio::Connection) -> Result<u64, QueueError> {
        let serialized = serde_json::to_string(self)?;
        conn.lpush("queue", serialized).await?;
        Ok(conn.incr("nonce", 1).await?)
    }
    async fn receive_job(conn: &mut redis::aio::Connection) -> Result<Job, QueueError> {
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
        let job = Job::new(JobKind::EncodeToSize, Some(Video {
            url: VideoURI::Url("https://cdn.discordapp.com/attachments/685197521953488994/1046181272319438969/edit-edit-edit-edit-edit-edit-edit-edit-edit-edit-out.mp4".to_string()),
            id: "sgfdvgsfsgvfsgvvd".to_owned(),
            filename: "toz.mp4".to_owned(),
        }), EncodeParameters::EncodeToSize(EncodeToSizeParameters {
            target_size: 7 * 2_u32.pow(20),
        }));

        println!("{}", job.send_job(&mut con).await.unwrap());

        // println!("{:?}", Job::receive_job(&mut con).await.unwrap());


        assert_eq!(1, 4);
    }
}
