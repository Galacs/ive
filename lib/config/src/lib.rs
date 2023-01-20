use s3::{creds::Credentials, region::Region, Bucket};
use std::env;

pub fn get_s3_bucket() -> Bucket {
    let creds = Credentials::new(Some("minioadmin"), Some("minioadmin"), None, None, None).unwrap();

    let bucket = Bucket::new(
        "ive",
        Region::Custom {
            region: "my-store".to_owned(),
            endpoint: env::var("IVE_S3_URL").expect("Expected an s3 url in the environment"),
        },
        creds,
    )
    .unwrap()
    .with_path_style();

    bucket
}

pub fn get_redis_client() -> redis::Client {
    redis::Client::open(env::var("IVE_REDIS_URL").expect("Expected a redis url in the environment")).unwrap()
}