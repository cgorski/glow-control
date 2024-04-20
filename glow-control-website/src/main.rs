use std::env;
use std::path::Path;

use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_s3 as s3;
use aws_sdk_s3::types::ObjectCannedAcl;
use futures::future::{BoxFuture, FutureExt};
use mime_guess::from_path;
use s3::Client;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        println!("Usage: glow-control-website <website_directory> <s3_bucket_name>");
        std::process::exit(1);
    }

    let directory = &args[1];
    let bucket_name = &args[2];

    // load sdkconfig from environment
    let config = aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await;
    let client = s3::Client::new(&config);

    upload_directory(client, directory.to_string(), bucket_name.to_string()).await?;

    println!("Website deployed successfully!");

    Ok(())
}

fn upload_directory(
    client: Client,
    directory: String,
    bucket_name: String,
) -> BoxFuture<'static, Result<()>> {
    async move {
        let paths = match std::fs::read_dir(directory) {
            Ok(paths) => paths,
            Err(e) => return Err(anyhow::Error::new(e)),
        };

        for entry in paths {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => return Err(anyhow::Error::new(e)),
            };
            let path = entry.path();

            if path.is_dir() {
                // Recursively call upload_directory
                upload_directory(
                    client.clone(),
                    path.to_str().unwrap().to_string(),
                    bucket_name.clone(),
                )
                .await?;
            } else {
                upload_file(&client, &path, &bucket_name).await?;
            }
        }

        Ok(())
    }
    .boxed() // Box the future
}

async fn upload_file(client: &Client, file_path: &Path, bucket_name: &str) -> Result<()> {
    let file_name = file_path.to_str().unwrap().replace("\\", "/");

    let content = tokio::fs::read(file_path).await?;

    let content_type = from_path(&file_name).first_or_octet_stream().to_string();

    let resp = client
        .put_object()
        .acl(ObjectCannedAcl::PublicRead)
        .bucket(bucket_name)
        .key(&file_name)
        .content_type(content_type)
        .body(content.into())
        .send()
        .await?;

    println!("Uploaded file to S3: {:?}", resp);

    Ok(())
}
