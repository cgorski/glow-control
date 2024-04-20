use std::collections::HashSet;
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3 as s3;
use aws_sdk_s3::types::{Delete, ObjectCannedAcl, ObjectIdentifier};
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

    // Load SDK config from environment
    let config = aws_config::load_defaults(BehaviorVersion::v2023_11_09()).await;
    let client = s3::Client::new(&config);

    // List all objects in the bucket before uploading
    let existing_objects = list_objects(&client, bucket_name).await?;

    // Initialize an empty HashSet to track uploaded files
    let uploaded_files = Arc::new(Mutex::new(HashSet::new()));

    // Upload directory and track uploaded files
    upload_directory(
        client.clone(),
        directory.to_string(),
        bucket_name.to_string(),
        uploaded_files.clone(),
    )
    .await?;

    let uploaded_files_lock = uploaded_files.lock().unwrap();

    let objects_to_remove: HashSet<_> = existing_objects
        .difference(&*uploaded_files_lock) // Use deref (*) to get the HashSet from the MutexGuard.
        .cloned()
        .collect();

    // Remove objects that weren't uploaded in this run
    if !objects_to_remove.is_empty() {
        delete_objects(&client, bucket_name, &objects_to_remove).await?;
    }

    println!("Website deployed successfully!");

    Ok(())
}

async fn list_objects(client: &Client, bucket_name: &str) -> Result<HashSet<String>> {
    let mut objects_set = HashSet::new();
    let mut response = client
        .list_objects_v2()
        .bucket(bucket_name.to_owned())
        .max_keys(100) // In this example, go 10 at a time.
        .into_paginator()
        .send();

    while let Some(result) = response.next().await {
        let resp = result?;

        for object in resp.contents.unwrap_or_default() {
            if let Some(key) = object.key {
                objects_set.insert(key);
            }
        }
    }

    Ok(objects_set)
}

async fn delete_objects(
    client: &Client,
    bucket_name: &str,
    objects_to_remove: &HashSet<String>,
) -> Result<()> {
    let objects: Vec<ObjectIdentifier> = objects_to_remove
        .iter()
        .map(|key| ObjectIdentifier::builder().key(key).build().unwrap())
        .collect();

    if !objects.is_empty() {
        client
            .delete_objects()
            .bucket(bucket_name)
            .delete(
                Delete::builder()
                    .set_objects(Some(objects))
                    .build()
                    .map_err(|e| anyhow!("error: {:#?}", e))?,
            )
            .send()
            .await?;
    }

    Ok(())
}
fn upload_directory(
    client: Client,
    directory: String,
    bucket_name: String,
    uploaded_files: Arc<Mutex<HashSet<String>>>,
) -> BoxFuture<'static, Result<()>> {
    async move {
        let paths = match std::fs::read_dir(&directory) {
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
                upload_directory(
                    client.clone(),
                    path.to_str().unwrap().to_string(),
                    bucket_name.clone(),
                    uploaded_files.clone(),
                )
                .await?;
            } else {
                upload_file(&client, &path, &bucket_name, uploaded_files.clone()).await?;
            }
        }

        Ok(())
    }
    .boxed()
}
async fn upload_file(
    client: &Client,
    file_path: &Path,
    bucket_name: &str,
    uploaded_files: Arc<Mutex<HashSet<String>>>,
) -> Result<()> {
    let file_name = file_path.to_str().unwrap().replace("\\", "/");

    let content = tokio::fs::read(file_path).await?;

    let content_type = from_path(&file_name).first_or_octet_stream().to_string();

    client
        .put_object()
        .acl(ObjectCannedAcl::PublicRead)
        .bucket(bucket_name)
        .key(&file_name)
        .content_type(content_type)
        .body(content.into())
        .send()
        .await?;

    let mut files = uploaded_files.lock().unwrap();
    files.insert(file_name);

    Ok(())
}
