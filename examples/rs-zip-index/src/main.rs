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

use anyhow::{anyhow, Result};
use async_zip::tokio::read::fs::ZipFileReader;
use std::ops::Deref;
use std::sync::Arc;
use std::{
    env::current_dir,
    path::{Path, PathBuf},
};
use std::time::Instant;
use tokio::fs::{create_dir_all, File, OpenOptions};
use tokio_util::compat::{TokioAsyncWriteCompatExt};
use tokio::runtime;
use tokio::runtime::Handle;

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().len() > 1 {
        let mut args = std::env::args().skip(1);

        let input_str = args.next().ok_or(anyhow!("No input zip file specified."))?;
        let zip_path = Path::new(&input_str);
        //let archive =  Path::new("example.zip");
        let out_dir = args.next().unwrap_or("./".to_string());
        unzip_file(zip_path, out_dir).await;
    } else {
        eprintln!("Error input!");
        eprintln!("Usage: muti_extraction <input zip file> <out folder>");
    }
    Ok(())
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
    let handle=Handle::current();
    let now = Instant::now();
    let mut handles = Vec::with_capacity(10);
    let share_dir = Arc::new(out_dir);
    let reader = ZipFileReader::new(archive).await.expect("Failed to read zip file");
    let share_reader = Arc::new(reader);
    for i in 0..share_reader.file().entries().len() {
        handles.push(tokio::spawn(write_entity(i, share_dir.clone(), share_reader.clone())));
    }

    futures::future::join_all(handles).await;
    let elapsed_time = now.elapsed();
    println!("Unzip file take {} seconds.", elapsed_time.as_secs());
}

async fn write_entity(index: usize, out_dir: Arc<String>, reader: Arc<ZipFileReader>) {
    println!("task id:{},runtime id:{}", tokio::task::id(),tokio::runtime::Handle::current().id());
    let dir = out_dir.deref();
    let entry = reader.file().entries().get(index).unwrap();
    let out_dir = Path::new(dir);
    let path = out_dir.join(sanitize_file_path(entry.filename().as_str().unwrap()));

    let entry_is_dir = entry.dir().unwrap();

    let entry_reader = reader.reader_without_entry(index).await.expect("Failed to read ZipEntry");

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
