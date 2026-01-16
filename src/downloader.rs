use std::fs::File;
use std::io::Write;
use std::path::Path;

use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header::{HeaderMap, HeaderValue, RANGE, CONTENT_LENGTH};

use crate::error::{Error, Result};
use crate::formats::Format;

const CHUNK_SIZE: u64 = 1024 * 1024;
#[allow(dead_code)]
const BUFFER_SIZE: usize = 8192;

/// Download a format to the specified output path
pub async fn download(format: &Format, output: &str, quiet: bool) -> Result<()> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert("Origin", HeaderValue::from_static("https://www.youtube.com"));
    default_headers.insert("Referer", HeaderValue::from_static("https://www.youtube.com/"));
    
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .default_headers(default_headers)
        .build()?;

    let total_size = get_content_length(&client, &format.url).await?;

    let _output_path = Path::new(output);
    let temp_path = format!("{}.part", output);
    let temp_file_path = Path::new(&temp_path);

    let mut downloaded: u64 = 0;
    let mut file = if temp_file_path.exists() {
        let metadata = std::fs::metadata(&temp_path)?;
        downloaded = metadata.len();
        
        if downloaded >= total_size {
            std::fs::rename(&temp_path, output)?;
            return Ok(());
        }

        if !quiet {
            println!(
                "{} Resuming download from {}",
                style("[67]").cyan().bold(),
                format_size(downloaded)
            );
        }

        std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&temp_path)?
    } else {
        File::create(&temp_path)?
    };

    let progress = if quiet {
        ProgressBar::hidden()
    } else {
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .unwrap()
                .progress_chars("█▓░"),
        );
        pb.set_position(downloaded);
        pb
    };

    while downloaded < total_size {
        let end = std::cmp::min(downloaded + CHUNK_SIZE - 1, total_size - 1);
        
        let mut headers = HeaderMap::new();
        headers.insert(
            RANGE,
            HeaderValue::from_str(&format!("bytes={}-{}", downloaded, end)).unwrap(),
        );

        let response = client
            .get(&format.url)
            .headers(headers)
            .send()
            .await?;

        if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(Error::DownloadFailed(format!(
                "HTTP {} while downloading",
                response.status()
            )));
        }

        let bytes = response.bytes().await?;
        file.write_all(&bytes)?;
        
        downloaded += bytes.len() as u64;
        progress.set_position(downloaded);
    }

    progress.finish_with_message("Download complete");

    drop(file);
    std::fs::rename(&temp_path, output)?;

    Ok(())
}

/// Get content length from URL via HEAD request
async fn get_content_length(client: &reqwest::Client, url: &str) -> Result<u64> {
    let response = client.head(url).send().await;
    
    if let Ok(resp) = response {
        if resp.status().is_success() {
            if let Some(len) = resp.headers().get(CONTENT_LENGTH) {
                if let Ok(len_str) = len.to_str() {
                    if let Ok(size) = len_str.parse::<u64>() {
                        return Ok(size);
                    }
                }
            }
        }
    }

    let mut headers = HeaderMap::new();
    headers.insert(RANGE, HeaderValue::from_static("bytes=0-0"));
    
    let response = client
        .get(url)
        .headers(headers)
        .send()
        .await?;

    if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
        return Err(Error::DownloadFailed(format!(
            "HTTP {} when checking content length",
            response.status()
        )));
    }

    if let Some(range) = response.headers().get("content-range") {
        if let Ok(range_str) = range.to_str() {
            if let Some(pos) = range_str.rfind('/') {
                if let Ok(size) = range_str[pos + 1..].parse::<u64>() {
                    return Ok(size);
                }
            }
        }
    }

    let response = client.get(url).send().await?;
    
    if !response.status().is_success() {
        return Err(Error::DownloadFailed(format!(
            "HTTP {} when fetching content",
            response.status()
        )));
    }
    
    response
        .content_length()
        .ok_or_else(|| Error::DownloadFailed("Could not determine file size".to_string()))
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
