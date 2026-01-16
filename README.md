# 67

```
  ████████   ██████████
 ███░░░░███ ░███░░░░███
░███   ░░░  ░░░    ███ 
░█████████        ███  
░███░░░░███      ███   
░███   ░███     ███    
░░████████     ███     
 ░░░░░░░░     ░░░      
```

**A fast, focused YouTube downloader CLI written in Rust.**

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Features

- 🚀 **Fast** - Direct InnerTube API, no browser overhead
- 📦 **Batch downloads** - Concurrent downloads from URL lists
- 📝 **Transcript extraction** - Extract subtitles/captions without downloading video
- 🎨 **Cinematic banner** - Rainbow gradient ASCII art
- 📊 **Format selection** - Best, worst, audio-only, video-only, or specific format IDs
- ⏸️ **Resume support** - Interrupted downloads continue via `.part` files
- 🔇 **Quiet mode** - For scripting and automation

## Quick Start

```bash
# Download a video
67 "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# List available formats
67 -F VIDEO_ID

# Download audio only
67 --audio-only VIDEO_ID

# Batch download with 4 concurrent jobs
67 -b urls.txt -j 4 -O ./downloads
```

## Installation

```bash
git clone https://github.com/YOUR_USERNAME/67.git
cd 67
cargo build --release

# Binary at target/release/67
```

## Usage

```bash
# Download best quality
67 VIDEO_ID
67 "https://www.youtube.com/watch?v=VIDEO_ID"
67 "https://youtu.be/VIDEO_ID"

# List formats
67 -F VIDEO_ID

# Select specific format
67 -f 22 VIDEO_ID
67 -f bestaudio VIDEO_ID
67 -f bestvideo VIDEO_ID

# Audio only
67 --audio-only VIDEO_ID

# Custom output
67 -o "my_video.mp4" VIDEO_ID

# Batch download
67 -b urls.txt -j 4 -O ./downloads

# Quiet mode (for scripts)
67 -q VIDEO_ID

# Extract transcripts (no download)
67 -M VIDEO_ID
```

## Transcript Extraction

Extract subtitles/captions to JSON without downloading the video:

```bash
67 -M VIDEO_ID
67 -M "https://www.youtube.com/watch?v=VIDEO_ID"
67 -M VIDEO_ID -o transcripts.json
```

Output format:

```json
{
  "video_id": "dQw4w9WgXcQ",
  "title": "Video Title",
  "channel": "Channel Name",
  "transcripts": [
    {
      "language": "English",
      "language_code": "en",
      "is_auto_generated": false,
      "segments": [
        {"start": 1.36, "duration": 1.68, "text": "Hello world"}
      ]
    }
  ]
}
```

## Format Selection

| Selector | Description |
|----------|-------------|
| `best` | Best muxed format (video+audio) |
| `bestaudio` | Best audio-only format |
| `bestvideo` | Best video-only format |
| `worst` | Lowest quality muxed format |
| `137` | Specific itag (from `-F` listing) |

## Batch Downloads

Create a file with URLs (one per line):

```bash
# urls.txt
https://youtube.com/watch?v=VIDEO1
https://youtube.com/watch?v=VIDEO2
# Comments are ignored
VIDEO3
```

Download with concurrency:

```bash
67 -b urls.txt -j 4 -O ./downloads
```

## How It Works

```
URL → Video ID → InnerTube API → Parse Formats → Concurrent Download
```

1. **URL Parsing** - Extracts video ID from any YouTube URL format
2. **InnerTube API** - Direct API call (Android client) bypasses throttling
3. **Format Selection** - Parses `streamingData.formats` and `adaptiveFormats`
4. **Download** - Chunked HTTP with resume support and progress bar

## Project Structure

```
src/
├── main.rs        # CLI entry point & batch processing
├── banner.rs      # Cinematic ASCII banner
├── extractor.rs   # InnerTube API extraction  
├── formats.rs     # Format parsing and selection
├── downloader.rs  # HTTP download with progress
├── metadata.rs    # Transcript extraction
└── error.rs       # Error types
```

## Dependencies

- **reqwest** - Async HTTP client with connection pooling
- **tokio** - Async runtime
- **futures** - Concurrent stream processing
- **tui-banner** - Cinematic terminal banners
- **clap** - CLI argument parsing
- **indicatif** - Progress bars
- **console** - Terminal styling

## Why "67"?

Short. Memorable. Slightly mysterious. 🎲

## License

MIT
