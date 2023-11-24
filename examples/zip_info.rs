// Copyright (c) 2021 Harry [Majored] [hello@majored.pw]
// MIT License (https://github.com/Majored/rs-async-zip/blob/main/LICENSE)

#[tokio::main]
async fn main() {
    if let Err(err) = inner::run().await {
        eprintln!("Error: {}", err);
        eprintln!("Usage: zip_info <input zip file>");
        std::process::exit(1);
    }
}

mod inner {

    use async_zip::base::*;
    use async_zip::{Compression, ZipEntryBuilder};

    use std::path::{Path, PathBuf};

    use anyhow::{anyhow, bail, Result};
    use async_zip::error::ZipError;
    use async_zip::tokio::read::fs::ZipFileReader;
    use futures_util::io::AsyncReadExt;
    use tokio::fs::File;

    pub(crate) async fn run() -> Result<()> {
        if std::env::args().len() > 1 {
            let mut args = std::env::args().skip(1);

            let input_str = args.next().ok_or(anyhow!("No input zip file specified."))?;
            let zip_path = Path::new(&input_str);

            if !zip_path.exists() {
                bail!("The input file specified doesn't exist.");
            }

            let mut reader = ZipFileReader::new(zip_path).await.expect("Failed to read zip file!");
            println!("file-count:{0}", reader.file().entries().len());
            for index in 0..reader.file().entries().len() {
                let entry = reader.file().entries().get(index).unwrap();

                let entry_is_dir = entry.dir().unwrap();

                let mut entry_reader = reader.reader_without_entry(index).await.expect("Failed to read ZipEntry");

                if entry_is_dir {
                    // The directory may have been created if iteration is out of order.
                    println!("dir-name:{0}", entry.filename().as_str().unwrap());
                } else {
                    println!("file-name:{0}", entry.filename().as_str().unwrap());
                    println!("compress-method:{:?}", entry.compression());
                    println!("header-size:{0}", entry.header_size());
                    println!("file-name-length:{0}", entry.filename().as_bytes().len());
                    println!("header-offset:{0}", entry.header_offset());
                    println!("uncompressed-size:{0}", entry.uncompressed_size());
                    println!("compress-size:{0}", entry.compressed_size());
                    for extra_field in entry.extra_fields() {
                        println!("extra_filed:{:?}", extra_field);
                    }
                    println!("compress-comment:{0}", entry.comment().as_str().unwrap());
                }
            }
        } else {
            eprintln!("Please input zip file!");
        }

        Ok(())
    }
}
