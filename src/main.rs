mod banner;
mod extractor;
mod downloader;
mod error;
mod formats;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::Parser;
use console::style;
use futures::stream::{self, StreamExt};
use tokio::sync::Semaphore;

use error::{Error, Result};

#[derive(Parser, Debug, Clone)]
#[command(name = "67")]
#[command(author = "6.7")]
#[command(version = "0.1.0")]
#[command(about = "A focused, efficient YouTube downloader", long_about = None)]
struct Args {
    /// YouTube URL or video ID (not required if using --batch-file)
    #[arg(required_unless_present = "batch_file")]
    url: Option<String>,

    /// Output filename (default: video title)
    #[arg(short, long)]
    output: Option<String>,

    /// Output directory for downloads (used with --batch-file)
    #[arg(short = 'O', long, default_value = ".")]
    output_dir: PathBuf,

    /// List available formats without downloading
    #[arg(short = 'F', long)]
    list_formats: bool,

    /// Select format by ID (e.g., "22" for 720p, "best", "bestaudio", "bestvideo")
    #[arg(short, long, default_value = "best")]
    format: String,

    /// Download audio only
    #[arg(short, long)]
    audio_only: bool,

    /// Be quiet (minimal output)
    #[arg(short, long)]
    quiet: bool,

    /// Batch file containing URLs (one per line)
    #[arg(short, long)]
    batch_file: Option<PathBuf>,

    /// Number of concurrent downloads
    #[arg(short, long, default_value = "3")]
    jobs: usize,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{} {}", style("Error:").red().bold(), e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args = Args::parse();

    if !args.quiet {
        banner::print_banner();
    }

    if let Some(batch_file) = &args.batch_file {
        return run_batch(batch_file, &args).await;
    }

    let url = args.url.as_ref().unwrap();
    download_single(url, &args, None).await
}

async fn run_batch(batch_file: &PathBuf, args: &Args) -> Result<()> {
    let content = std::fs::read_to_string(batch_file)
        .map_err(|e| Error::Io(e))?;
    
    let urls: Vec<String> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect();

    if urls.is_empty() {
        return Err(Error::ExtractionFailed("No URLs found in batch file".to_string()));
    }

    let total = urls.len();
    let jobs = args.jobs.max(1).min(10);

    if !args.quiet {
        println!("{} Processing {} URLs with {} concurrent jobs", 
            style("[67]").cyan().bold(), 
            style(total).yellow(),
            style(jobs).yellow()
        );
    }

    if !args.output_dir.exists() {
        std::fs::create_dir_all(&args.output_dir)
            .map_err(|e| Error::Io(e))?;
    }

    let completed = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let semaphore = Arc::new(Semaphore::new(jobs));

    let _results: Vec<_> = stream::iter(urls.into_iter().enumerate())
        .map(|(idx, url)| {
            let args = args.clone();
            let sem = semaphore.clone();
            let completed = completed.clone();
            let failed = failed.clone();
            
            async move {
                let _permit = sem.acquire().await.unwrap();
                
                let result = download_single(&url, &args, Some(idx + 1)).await;
                
                match &result {
                    Ok(_) => {
                        completed.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(e) => {
                        failed.fetch_add(1, Ordering::SeqCst);
                        if !args.quiet {
                            eprintln!("{} [{}] Failed: {}", 
                                style("[67]").red().bold(),
                                idx + 1,
                                e
                            );
                        }
                    }
                }
                
                result
            }
        })
        .buffer_unordered(jobs)
        .collect()
        .await;

    let success = completed.load(Ordering::SeqCst);
    let failures = failed.load(Ordering::SeqCst);
    
    if !args.quiet {
        println!();
        println!("{} Batch complete: {} succeeded, {} failed", 
            style("[67]").cyan().bold(),
            style(success).green(),
            if failures > 0 { style(failures).red() } else { style(failures).dim() }
        );
    }

    if failures > 0 && success == 0 {
        Err(Error::DownloadFailed("All downloads failed".to_string()))
    } else {
        Ok(())
    }
}

async fn download_single(url: &str, args: &Args, batch_index: Option<usize>) -> Result<()> {
    let prefix = batch_index
        .map(|i| format!("[{}]", i))
        .unwrap_or_default();

    let video_id = extractor::parse_video_id(url)?;
    
    let video_info = extractor::extract_video_info(&video_id).await?;

    if args.list_formats {
        formats::print_formats(&video_info.formats);
        return Ok(());
    }

    let selected = formats::select_format(&video_info.formats, &args.format, args.audio_only)?;
    
    let output = if let Some(ref out) = args.output {
        out.clone()
    } else {
        let safe_title = sanitize_filename(&video_info.title);
        let filename = format!("{}.{}", safe_title, selected.extension());
        
        if batch_index.is_some() {
            args.output_dir.join(&filename).to_string_lossy().to_string()
        } else {
            filename
        }
    };

    if std::env::var("DEBUG").is_ok() {
        eprintln!("DEBUG URL: {}", &selected.url[..std::cmp::min(200, selected.url.len())]);
    }

    let quiet_download = args.quiet || batch_index.is_some();
    downloader::download(&selected, &output, quiet_download).await?;

    if !args.quiet {
        println!("{} {} Downloaded: {}", 
            style("[67]").cyan().bold(), 
            prefix,
            style(&output).yellow()
        );
    }

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}
