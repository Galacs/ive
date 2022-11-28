use std::path::Path;

use queue::Queue;
use models::{Job, EncodeParameters};
use redis::Commands;
use tokio::fs;

#[tokio::main]

async fn main() {
    let mut client = config::get_redis_client();
    let mut con = client.get_async_connection().await.unwrap();

    loop {
        let a = Job::receive_job(&mut con).await.unwrap();
        let video = a.video.unwrap();

        // Define working directory and destination filepath
        let dir = Path::new("tmpfs").join(format!("{}", &video.id));
        let dir = std::env::current_dir().unwrap().join(dir);
        let dest_file = dir.join(&format!("edit-{}", &video.filename));

        // Creating working directory
        fs::create_dir(&dir).await.unwrap();

        let channel = format!("progress:{}", video.id);

        let _ : () = client.publish(&channel, "starting").unwrap();

        let params = match &a.params {
            EncodeParameters::EncodeToSize(p) => p,
        };

        let _ = ffedit::encoding::encode_to_size_new(&video, params).await;

        let dir = ffedit::encoding::get_working_dir(&video.id).unwrap();
        tokio::fs::remove_dir_all(dir).await.unwrap();

        let _ : () = client.publish(&channel, "done").unwrap();
    }

}
