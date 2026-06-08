use std::{fs::{self, File}, io::{BufRead, Cursor, Write}};

use flate2::read::GzDecoder;

#[derive(Debug)]
pub enum DownloadError {
    HTTPError(reqwest::Error),
    IOError(std::io::Error),
}

pub async fn download_file(url: &str, output_path: &str) -> Result<(), DownloadError>{
    let response = reqwest::get(url).await.map_err(|e| DownloadError::HTTPError(e))?;
    let body = response.bytes().await.map_err(|e| DownloadError::HTTPError(e))?;
    let mut out = File::create(output_path).unwrap();
    out.write_all(&mut body.as_ref()).map_err(|e| DownloadError::IOError(e))
}

pub async fn unzip_file(src: &str, dest: &str) -> Result<(), std::io::Error> {
    let src_file = File::open(src)?;
    let mut decoder = GzDecoder::new(src_file);
    let mut dest_file = File::create(&dest)?;
    std::io::copy(&mut decoder, &mut dest_file)?;
    Ok(())
}

pub async fn list_commoncrawl_files() -> Result<Vec<String>, DownloadError> {
    let url = "https://data.commoncrawl.org/crawl-data/CC-MAIN-2026-21/wet.paths.gz";
    let output_path = "wet.paths.gz";
    let dest = "wet.paths";
    download_file(url, output_path).await?;
    unzip_file(output_path, dest).await.unwrap();
    std::fs::remove_file(output_path).unwrap();

    let file_bytes = fs::read(dest).unwrap();
    let reader = Cursor::new(file_bytes);

    let paths = reader.lines().map(|line| line.unwrap()).collect::<Vec<String>>();
    std::fs::remove_file(dest).unwrap();

    Ok(paths)
}

pub async fn test_download() -> Result<(), DownloadError> {
    let paths = list_commoncrawl_files().await?;
    println!("Last 10 paths :");
    for path in paths.iter().rev().take(10) {
        println!("{}", path);
    }
    Ok(())
}
