use std::{
    fs::{self, File},
    io::{BufRead, Cursor, Write},
};

use flate2::read::MultiGzDecoder;

#[derive(Debug)]
pub enum DownloadError {
    HTTPError(reqwest::Error),
    IOError(std::io::Error),
}

pub async fn download_file(url: &str, output_path: &str) -> Result<(), DownloadError> {
    let response = reqwest::get(url).await.map_err(DownloadError::HTTPError)?;
    let body = response.bytes().await.map_err(DownloadError::HTTPError)?;
    let dest_path = std::path::Path::new(output_path);
    std::fs::create_dir_all(dest_path.parent().unwrap()).map_err(DownloadError::IOError)?;
    let mut out = File::create(dest_path).unwrap();
    out.write_all(body.as_ref()).map_err(DownloadError::IOError)
}

pub async fn unzip_file(src: &str, dest: &str) -> Result<(), std::io::Error> {
    let src_file = File::open(src)?;
    let mut decoder = MultiGzDecoder::new(src_file);
    let dest_path = std::path::Path::new(dest);
    std::fs::create_dir_all(dest_path.parent().unwrap())?;
    let mut dest_file = File::create(dest_path)?;
    std::io::copy(&mut decoder, &mut dest_file)?;
    Ok(())
}

pub async fn list_commoncrawl_files(tmp_dir: &str) -> Result<Vec<String>, DownloadError> {
    let url = crate::tasks::WET_PATHS_URL;
    let output_path = format!("{}wet.paths.gz", tmp_dir);
    let dest = format!("{}wet.paths", tmp_dir);
    download_file(url, &output_path).await?;
    unzip_file(&output_path, &dest).await.unwrap();
    std::fs::remove_file(&output_path).unwrap();

    let file_bytes = fs::read(&dest).unwrap();
    let reader = Cursor::new(file_bytes);

    let paths = reader
        .lines()
        .map(|line| line.unwrap())
        .collect::<Vec<String>>();
    std::fs::remove_file(&dest).unwrap();

    Ok(paths)
}

pub async fn get_commoncrawl_file(link: &str, output_name: &str) -> Result<String, DownloadError> {
    let url = format!("https://data.commoncrawl.org/{}", link);
    let gz_file = format!("{}.warc.wet.gz", output_name);
    let dest = format!("{}.warc.wet", output_name);

    download_file(&url, &gz_file).await.unwrap();
    unzip_file(&gz_file, &dest).await.unwrap();
    std::fs::remove_file(&gz_file).unwrap();

    Ok(dest)
}

pub async fn test_download() -> Result<(), DownloadError> {
    let links = list_commoncrawl_files("./tests/data/").await?;
    println!("Last 10 links :");
    for (i, link) in links.iter().rev().take(10).enumerate() {
        println!("Downloading {}...", link);
        let output_name = format!("./tests/data/CC-MAIN-{}", i);
        get_commoncrawl_file(link, &output_name).await.unwrap();
    }
    Ok(())
}

pub fn test_download_sync() -> Result<(), DownloadError> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(test_download())
}
