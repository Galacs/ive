use std::path::Path;

use models::{EncodeParameters, Job, JobProgress, InteractionError};
use queue::Queue;
use redis::{Client, Commands};
use tokio::fs;

#[derive(Debug)]
enum ProcessError {
    NoVideo,
    File(std::io::Error),
    Serde(serde_json::Error),
    Redis(redis::RedisError),
    Error,
}

impl From<std::io::Error> for ProcessError {
    fn from(error: std::io::Error) -> Self {
        ProcessError::File(error)
    }
}

impl From<serde_json::Error> for ProcessError {
    fn from(error: serde_json::Error) -> Self {
        ProcessError::Serde(error)
    }
}

impl From<redis::RedisError> for ProcessError {
    fn from(error: redis::RedisError) -> Self {
        ProcessError::Redis(error)
    }
}

impl From<InteractionError> for ProcessError {
    fn from(_: InteractionError) -> Self {
        ProcessError::Error
    }
}

async fn process_job(job: Job, client: &mut Client) -> Result<(), ProcessError> {
    let video = job.video.ok_or(ProcessError::NoVideo)?;

    // Define working directory and destination filepath
    let dir = Path::new("tmpfs").join(format!("{}", &video.id));
    let dir = std::env::current_dir()?.join(dir);

    // Creating working directory
    fs::create_dir(&dir).await?;

    let channel = format!("progress:{}", video.id);

    let str = serde_json::to_string(&JobProgress::Started)?;
    let _: () = client.publish(&channel, str)?;

    let res = match &job.params {
        EncodeParameters::EncodeToSize(p) => ffedit::encode_to_size(&video, p).await,
        EncodeParameters::Cut(p) => ffedit::cut(&video, p).await,
        EncodeParameters::Remux(p) => ffedit::remux(&video, p).await,
    };

    match res {
        Err(err) => {
            let _: () = client.publish(&channel, serde_json::to_string(&JobProgress::Error(format!("{}", err)))?)?;
            return Err(ProcessError::Error)
        },
        Ok(_) => {},
    }

    let dir = ffedit::get_working_dir(&video.id)?;
    tokio::fs::remove_dir_all(dir).await?;

    let file_extension = match job.params {
        EncodeParameters::Remux(container) => container.container.get_file_extension(),
        _ => "mp4".to_owned(),
    };

    let str = serde_json::to_string(&JobProgress::Done(file_extension.to_owned()))?;
    let _: () = client.publish(&channel, str)?;
    Ok(())
}

#[tokio::main]

async fn main() {
    let mut client = config::get_redis_client();
    let mut con = client.get_async_connection().await.unwrap();

    // Create tmp folder
    if !Path::new("tmpfs").exists() {
        if let Err(why) = fs::create_dir("tmpfs").await {
            panic!("Can't create tmp dir: {}", why);
        }
    }

    loop {
        let job = Job::receive_job(&mut con).await;
        let job = match job {
            Ok(j) => j,
            Err(err) => {
                println!("{:?}", err);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }
        };
        if let Err(why) = process_job(job, &mut client).await {
            println!("Processing error: {:?}", why);
        }
    }
}
