use s3::{creds, region};

use creds::Credentials;
use region::Region;
use s3::error::S3Error;
use s3::Bucket;

pub fn get_s3_bucket() -> Bucket {
    let creds = Credentials::new(Some("minioadmin"), Some("minioadmin"), None, None, None).unwrap();

    let bucket = Bucket::new(
        "ive",
        Region::Custom {
            region: "my-store".to_owned(),
            endpoint: "http://127.0.0.1:9000".to_owned(),
        },
        creds,
    )
    .unwrap()
    .with_path_style();

    bucket
}
