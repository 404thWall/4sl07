use std::{fs::File, io::Write};

use flate2::read::GzDecoder;

#[derive(Debug)]
pub enum DownloadError {
    HTTPError(reqwest::Error),
    IOError(std::io::Error),
}

pub async fn download_file(url: &str, output_path: &str) -> Result<(), DownloadError>{
    // let client = reqwest::Client::new();
    // let response = client
    //     .get(url)
    //     .header(reqwest::header::ACCEPT_ENCODING, "identity")
    //     .send()
    //     .await
    //     .map_err(|e| DownloadError::HTTPError(e))?
    //     .error_for_status()
    //     .map_err(|e| DownloadError::HTTPError(e))?;

    // let body = response
    //     .bytes()
    //     .await
    //     .map_err(|e| DownloadError::HTTPError(e))?;

    // let mut out = File::create(output_path).map_err(|e| DownloadError::IOError(e))?;
    // out.write_all(body.as_ref()).map_err(|e| DownloadError::IOError(e))

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

pub async fn list_commoncrawl_files() {
    let url = "https://data.commoncrawl.org/crawl-data/CC-MAIN-2026-21/wet.paths.gz";
    let output_path = "wet.paths.gz";
    let dest = "wet.paths";
    download_file(url, output_path).await.unwrap();
    unzip_file(output_path, dest).await.unwrap();
    std::fs::remove_file(output_path).unwrap();
}

pub async fn test_download() -> Result<(), DownloadError> {
    list_commoncrawl_files().await;
    Ok(())
}
