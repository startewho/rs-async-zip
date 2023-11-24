//! Demonstrates how to safely extract everything from a ZIP file.
//!
//! Extracting zip files from untrusted sources without proper sanitization
//! could be exploited by directory traversal attacks.
//! <https://en.wikipedia.org/wiki/Directory_traversal_attack#Archives>
//!
//! This example tries to minimize that risk by following the implementation from
//! Python's Standard Library.
//! <https://docs.python.org/3/library/zipfile.html#zipfile.ZipFile.extract>
//! <https://github.com/python/cpython/blob/ac0a19b62ae137c2c9f53fbba8ba3f769acf34dc/Lib/zipfile.py#L1662>
//!

use std::{
    env::current_dir,
    path::{Path, PathBuf},
};
use std::sync::Arc;

use async_zip::tokio::read::fs::ZipFileReader;
use tokio::fs::{create_dir_all, File, OpenOptions};
use tokio::task;
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[tokio::main]
async fn main() {
    let archive =  Path::new("example.zip");
    let out_dir = ".".to_string();

    unzip_file(archive, out_dir).await;
}

/// Returns a relative path without reserved names, redundant separators, ".", or "..".
fn sanitize_file_path(path: &str) -> PathBuf {
    // Replaces backwards slashes
    path.replace('\\', "/")
        // Sanitizes each component
        .split('/')
        .map(sanitize_filename::sanitize)
        .collect()
}

/// Extracts everything from the ZIP archive to the output directory
async fn unzip_file(archive: &Path, out_dir: String) {

    let mut handles = Vec::with_capacity(10);
    let out_vec=Arc::new(out_dir.into_bytes());
    let  reader = ZipFileReader::new(archive).await.expect("Failed to read zip file");
    for i in 0..reader.file().entries().len() {
        handles.push(tokio::spawn(write_entity(i,out_vec.clone(),reader.clone())));
    }
    futures::future::join_all(handles).await;

}

async fn write_entity(index: usize,out_dir:Arc<Vec<u8>>,reader: ZipFileReader) {


    let entry = reader.file().entries().get(index).unwrap();
    let out_dir = current_dir().expect("Failed to get current working directory");
    let path = out_dir.join(sanitize_file_path(entry.filename().as_str().unwrap()));

    let entry_is_dir = entry.dir().unwrap();

    let   entry_reader = reader.reader_without_entry(index).await.expect("Failed to read ZipEntry");

    if entry_is_dir {
        // The directory may have been created if iteration is out of order.
        if !path.exists() {
            create_dir_all(&path).await.expect("Failed to create extracted directory");
        }
    } else {
        // Creates parent directories. They may not exist if iteration is out of order
        // or the archive does not contain directory entries.
        let parent = path.parent().expect("A file entry should have parent directories");
        if !parent.is_dir() {
            create_dir_all(parent).await.expect("Failed to create parent directories");
        }
        let writer =
            OpenOptions::new().write(true).create(true).open(&path).await.expect("Failed to create extracted file");
        futures_util::io::copy(entry_reader, &mut writer.compat_write())
            .await
            .expect("Failed to copy to extracted file");

        // Closes the file and manipulates its metadata here if you wish to preserve its metadata from the archive.
    }
}
