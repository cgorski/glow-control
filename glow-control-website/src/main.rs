use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::{anyhow, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3 as s3;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};
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
    let config = aws_config::load_defaults(BehaviorVersion::v2024_03_28()).await;
    let client = s3::Client::new(&config);

    // List all objects in the bucket before uploading
    let existing_objects = list_objects(&client, bucket_name).await?;

    // Initialize an empty HashSet to track uploaded files
    let uploaded_files = Arc::new(Mutex::new(HashSet::new()));

    // Get full path of the website directory
    let directory = std::fs::canonicalize(directory)?;

    // Change working directory to the website directory
    std::env::set_current_dir(directory.clone())?;

    // Upload directory and track uploaded files
    upload_directory(
        client.clone(),
        directory.clone(),
        directory,
        bucket_name.to_string(),
        uploaded_files.clone(),
    )
    .await?;

    let objects_to_remove: HashSet<_> = {
        let uploaded_files_lock = uploaded_files.lock().unwrap();

        existing_objects
            .difference(&*uploaded_files_lock) // Use deref (*) to get the HashSet from the MutexGuard.
            .cloned()
            .collect()
    };

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
                println!("Found object: {}", key);
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
        println!("Deleting objects: {:#?}", objects);
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
    base_path: PathBuf,
    directory: PathBuf,
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
                    base_path.clone(),
                    path.clone(),
                    bucket_name.clone(),
                    uploaded_files.clone(),
                )
                .await?;
            } else {
                upload_file(
                    &client,
                    &base_path,
                    &path,
                    &bucket_name,
                    uploaded_files.clone(),
                )
                .await?;
            }
        }

        Ok(())
    }
    .boxed()
}

fn remove_base_path(base_path: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(base_path).unwrap_or(path).to_path_buf()
}
async fn upload_file(
    client: &Client,
    base_path: &Path,
    file_path: &Path,
    bucket_name: &str,
    uploaded_files: Arc<Mutex<HashSet<String>>>,
) -> Result<()> {
    let file_name = file_path.to_str().unwrap().replace('\\', "/");
    let s3_key = remove_base_path(base_path, file_path);
    let s3_key = s3_key.to_str().unwrap();

    println!("Uploading path: {} to s3 key: {}", file_name, s3_key);
    let content = tokio::fs::read(file_path).await?;

    let content_type = from_path(&file_name).first_or_octet_stream().to_string();

    client
        .put_object()
        .bucket(bucket_name)
        .key(s3_key)
        .content_type(content_type)
        .body(content.into())
        .send()
        .await?;

    let mut files = uploaded_files.lock().unwrap();
    files.insert(s3_key.to_string());

    Ok(())
}
