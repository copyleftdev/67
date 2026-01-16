use console::style;
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Format {
    pub format_id: String,
    pub url: String,
    pub container: String,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<u32>,
    pub bitrate: Option<u32>,
    pub filesize: Option<u64>,
    pub quality: String,
    pub quality_label: Option<String>,
    pub audio_quality: Option<String>,
    pub audio_sample_rate: Option<u32>,
    pub audio_channels: Option<u32>,
    pub is_audio_only: bool,
    pub is_video_only: bool,
}

impl Format {
    pub fn extension(&self) -> &str {
        match self.container.as_str() {
            "mp4" => "mp4",
            "webm" => "webm",
            "3gp" => "3gp",
            "m4a" => "m4a",
            _ if self.is_audio_only => "m4a",
            _ => "mp4",
        }
    }

    pub fn format_note(&self) -> String {
        let mut parts = Vec::new();

        if let Some(label) = &self.quality_label {
            parts.push(label.clone());
        } else if let Some(h) = self.height {
            parts.push(format!("{}p", h));
        }

        if let Some(fps) = self.fps {
            if fps > 30 {
                parts.push(format!("{}fps", fps));
            }
        }

        if self.is_audio_only {
            if let Some(aq) = &self.audio_quality {
                let quality = aq.replace("AUDIO_QUALITY_", "").to_lowercase();
                parts.push(format!("audio {}", quality));
            } else {
                parts.push("audio only".to_string());
            }
        } else if self.is_video_only {
            parts.push("video only".to_string());
        }

        parts.push(self.container.clone());

        if let Some(vc) = &self.video_codec {
            let short = shorten_codec(vc);
            if !short.is_empty() {
                parts.push(short);
            }
        }

        if let Some(ac) = &self.audio_codec {
            let short = shorten_codec(ac);
            if !short.is_empty() {
                parts.push(short);
            }
        }

        if let Some(size) = self.filesize {
            parts.push(format_size(size));
        } else if let Some(br) = self.bitrate {
            parts.push(format!("~{}k", br / 1000));
        }

        parts.join(", ")
    }

    /// Calculate a quality score for sorting (higher is better)
    pub fn quality_score(&self) -> i64 {
        let mut score: i64 = 0;

        // Resolution score
        if let Some(h) = self.height {
            score += (h as i64) * 1000;
        }

        // FPS bonus
        if let Some(fps) = self.fps {
            score += (fps as i64) * 10;
        }

        // Bitrate score
        if let Some(br) = self.bitrate {
            score += (br as i64) / 1000;
        }

        // Prefer formats with both audio and video
        if !self.is_audio_only && !self.is_video_only {
            score += 500000; // Big bonus for muxed formats
        }

        // Container preference (mp4 > webm)
        if self.container == "mp4" {
            score += 100;
        }

        score
    }

    /// Calculate audio quality score (higher is better)
    pub fn audio_quality_score(&self) -> i64 {
        let mut score: i64 = 0;

        // Audio quality
        if let Some(aq) = &self.audio_quality {
            score += match aq.as_str() {
                "AUDIO_QUALITY_HIGH" => 400,
                "AUDIO_QUALITY_MEDIUM" => 300,
                "AUDIO_QUALITY_LOW" => 200,
                _ => 100,
            };
        }

        // Sample rate
        if let Some(sr) = self.audio_sample_rate {
            score += (sr as i64) / 100;
        }

        // Bitrate
        if let Some(br) = self.bitrate {
            score += (br as i64) / 100;
        }

        // Channels
        if let Some(ch) = self.audio_channels {
            score += (ch as i64) * 50;
        }

        score
    }
}

fn shorten_codec(codec: &str) -> String {
    if codec.starts_with("avc1") {
        "h264".to_string()
    } else if codec.starts_with("av01") {
        "av1".to_string()
    } else if codec.starts_with("vp9") || codec.starts_with("vp09") {
        "vp9".to_string()
    } else if codec.starts_with("vp8") {
        "vp8".to_string()
    } else if codec.starts_with("mp4a") {
        "aac".to_string()
    } else if codec.starts_with("opus") {
        "opus".to_string()
    } else {
        String::new()
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB", bytes as f64 / KB as f64)
    } else {
        format!("{}B", bytes)
    }
}

/// Print available formats in a table
pub fn print_formats(formats: &[Format]) {
    println!();
    println!(
        "{:>6}  {:>10}  {:>5}  {:>8}  {:>10}  {}",
        style("ID").bold().underlined(),
        style("EXT").bold().underlined(),
        style("RES").bold().underlined(),
        style("FPS").bold().underlined(),
        style("SIZE").bold().underlined(),
        style("NOTE").bold().underlined(),
    );

    let mut sorted: Vec<&Format> = formats.iter().collect();
    sorted.sort_by(|a, b| b.quality_score().cmp(&a.quality_score()));

    for fmt in sorted {
        let res = if let Some(h) = fmt.height {
            format!("{}p", h)
        } else if fmt.is_audio_only {
            "audio".to_string()
        } else {
            "-".to_string()
        };

        let fps = fmt.fps.map(|f| f.to_string()).unwrap_or("-".to_string());
        
        let size = fmt.filesize
            .map(format_size)
            .or_else(|| fmt.bitrate.map(|b| format!("~{}k", b / 1000)))
            .unwrap_or("-".to_string());

        let note = fmt.format_note();

        let id_style = if fmt.is_audio_only {
            style(&fmt.format_id).blue()
        } else if fmt.is_video_only {
            style(&fmt.format_id).magenta()
        } else {
            style(&fmt.format_id).green()
        };

        println!(
            "{:>6}  {:>10}  {:>5}  {:>8}  {:>10}  {}",
            id_style,
            fmt.container,
            res,
            fps,
            size,
            style(note).dim(),
        );
    }

    println!();
    println!(
        "{} = muxed (video+audio)  {} = video only  {} = audio only",
        style("green").green(),
        style("magenta").magenta(),
        style("blue").blue(),
    );
}

/// Select a format based on format string
pub fn select_format(formats: &[Format], format_str: &str, audio_only: bool) -> Result<Format> {
    if formats.is_empty() {
        return Err(Error::NoFormats);
    }

    // If audio_only flag is set, select best audio
    if audio_only {
        return select_best_audio(formats);
    }

    match format_str.to_lowercase().as_str() {
        "best" => select_best(formats),
        "bestaudio" => select_best_audio(formats),
        "bestvideo" => select_best_video(formats),
        "worst" => select_worst(formats),
        _ => {
            // Try to find by format ID
            formats
                .iter()
                .find(|f| f.format_id == format_str)
                .cloned()
                .ok_or_else(|| Error::FormatNotFound(format_str.to_string()))
        }
    }
}

fn select_best(formats: &[Format]) -> Result<Format> {
    // Prefer muxed formats (video + audio combined)
    let muxed: Vec<&Format> = formats
        .iter()
        .filter(|f| !f.is_audio_only && !f.is_video_only)
        .collect();

    if !muxed.is_empty() {
        return muxed
            .into_iter()
            .max_by_key(|f| f.quality_score())
            .cloned()
            .ok_or(Error::NoFormats);
    }

    // Fall back to best video-only format
    formats
        .iter()
        .filter(|f| !f.is_audio_only)
        .max_by_key(|f| f.quality_score())
        .cloned()
        .ok_or(Error::NoFormats)
}

fn select_best_audio(formats: &[Format]) -> Result<Format> {
    formats
        .iter()
        .filter(|f| f.is_audio_only || f.audio_codec.is_some())
        .max_by_key(|f| f.audio_quality_score())
        .cloned()
        .ok_or(Error::FormatNotFound("No audio formats available".to_string()))
}

fn select_best_video(formats: &[Format]) -> Result<Format> {
    formats
        .iter()
        .filter(|f| !f.is_audio_only)
        .max_by_key(|f| f.quality_score())
        .cloned()
        .ok_or(Error::FormatNotFound("No video formats available".to_string()))
}

fn select_worst(formats: &[Format]) -> Result<Format> {
    let muxed: Vec<&Format> = formats
        .iter()
        .filter(|f| !f.is_audio_only && !f.is_video_only)
        .collect();

    if !muxed.is_empty() {
        return muxed
            .into_iter()
            .min_by_key(|f| f.quality_score())
            .cloned()
            .ok_or(Error::NoFormats);
    }

    formats
        .iter()
        .min_by_key(|f| f.quality_score())
        .cloned()
        .ok_or(Error::NoFormats)
}
