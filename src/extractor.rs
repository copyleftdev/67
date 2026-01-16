use regex::Regex;
use serde_json::{json, Value};

use crate::error::{Error, Result};
use crate::formats::Format;

// Android client - doesn't require n-parameter decryption or PO tokens
const ANDROID_USER_AGENT: &str = "com.google.android.youtube/20.10.38 (Linux; U; Android 11) gzip";
const INNERTUBE_API_KEY: &str = "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w";

#[derive(Debug)]
#[allow(dead_code)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub duration: Option<u64>,
    pub description: Option<String>,
    pub formats: Vec<Format>,
    pub thumbnail: Option<String>,
}

/// Parse video ID from various YouTube URL formats or raw ID
pub fn parse_video_id(input: &str) -> Result<String> {
    let input = input.trim();
    
    // Raw video ID (11 characters, alphanumeric + _ -)
    let id_regex = Regex::new(r"^[0-9A-Za-z_-]{11}$").unwrap();
    if id_regex.is_match(input) {
        return Ok(input.to_string());
    }

    // YouTube URL patterns
    let patterns = [
        // Standard watch URL
        r"(?:youtube\.com|youtu\.be)/(?:watch\?.*?v=|embed/|v/|shorts/|live/)?([0-9A-Za-z_-]{11})",
        // youtu.be short URL
        r"youtu\.be/([0-9A-Za-z_-]{11})",
    ];

    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(input) {
            if let Some(id) = caps.get(1) {
                return Ok(id.as_str().to_string());
            }
        }
    }

    Err(Error::InvalidUrl(input.to_string()))
}

/// Extract video information from YouTube using InnerTube API (Android client)
pub async fn extract_video_info(video_id: &str) -> Result<VideoInfo> {
    let client = reqwest::Client::builder()
        .user_agent(ANDROID_USER_AGENT)
        .build()?;

    // Use InnerTube API with Android client - bypasses n-parameter throttling
    let api_url = format!(
        "https://www.youtube.com/youtubei/v1/player?key={}&prettyPrint=false",
        INNERTUBE_API_KEY
    );

    let request_body = json!({
        "context": {
            "client": {
                "clientName": "ANDROID",
                "clientVersion": "20.10.38",
                "androidSdkVersion": 30,
                "userAgent": ANDROID_USER_AGENT,
                "osName": "Android",
                "osVersion": "11"
            }
        },
        "videoId": video_id,
        "playbackContext": {
            "contentPlaybackContext": {
                "html5Preference": "HTML5_PREF_WANTS"
            }
        },
        "contentCheckOk": true,
        "racyCheckOk": true
    });

    let response = client
        .post(&api_url)
        .header("Content-Type", "application/json")
        .header("X-YouTube-Client-Name", "3")
        .header("X-YouTube-Client-Version", "20.10.38")
        .body(request_body.to_string())
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(Error::ExtractionFailed(format!(
            "HTTP {} from InnerTube API",
            response.status()
        )));
    }

    let player_response: Value = response.json().await?;

    // Check playability
    check_playability(&player_response)?;

    // Extract video details
    let video_details = player_response
        .get("videoDetails")
        .ok_or_else(|| Error::ExtractionFailed("Missing videoDetails".to_string()))?;

    let title = video_details
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let channel = video_details
        .get("author")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let duration = video_details
        .get("lengthSeconds")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok());

    let description = video_details
        .get("shortDescription")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let thumbnail = video_details
        .get("thumbnail")
        .and_then(|t| t.get("thumbnails"))
        .and_then(|t| t.as_array())
        .and_then(|arr| arr.last())
        .and_then(|t| t.get("url"))
        .and_then(|u| u.as_str())
        .map(|s| s.to_string());

    // Extract formats from streaming data
    let streaming_data = player_response.get("streamingData");
    let mut formats = Vec::new();

    if let Some(sd) = streaming_data {
        // Regular formats (muxed video+audio)
        if let Some(fmts) = sd.get("formats").and_then(|f| f.as_array()) {
            for fmt in fmts {
                if let Some(format) = parse_format(fmt) {
                    formats.push(format);
                }
            }
        }

        // Adaptive formats (separate video/audio streams)
        if let Some(fmts) = sd.get("adaptiveFormats").and_then(|f| f.as_array()) {
            for fmt in fmts {
                if let Some(format) = parse_format(fmt) {
                    formats.push(format);
                }
            }
        }
    }

    if formats.is_empty() {
        return Err(Error::NoFormats);
    }

    Ok(VideoInfo {
        id: video_id.to_string(),
        title,
        channel,
        duration,
        description,
        formats,
        thumbnail,
    })
}

#[allow(dead_code)]
fn extract_player_response(html: &str) -> Result<Value> {
    // Pattern to find ytInitialPlayerResponse
    let patterns = [
        r"ytInitialPlayerResponse\s*=\s*(\{.+?\});",
        r"var\s+ytInitialPlayerResponse\s*=\s*(\{.+?\});",
    ];

    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(html) {
            if let Some(json_str) = caps.get(1) {
                // Try to find the end of the JSON object
                let json_text = json_str.as_str();
                if let Ok(value) = find_and_parse_json(json_text) {
                    return Ok(value);
                }
            }
        }
    }

    // Alternative: look for the JSON in script tags
    let script_re = Regex::new(r#"<script[^>]*>.*?ytInitialPlayerResponse\s*=\s*(\{.*?\});\s*(?:var|</script>)"#).unwrap();
    if let Some(caps) = script_re.captures(html) {
        if let Some(json_str) = caps.get(1) {
            if let Ok(value) = serde_json::from_str(json_str.as_str()) {
                return Ok(value);
            }
        }
    }

    Err(Error::ExtractionFailed(
        "Could not find ytInitialPlayerResponse in page".to_string(),
    ))
}

#[allow(dead_code)]
fn find_and_parse_json(text: &str) -> Result<Value> {
    // Find matching braces to extract complete JSON
    let mut depth = 0;
    let mut end_pos = 0;
    
    for (i, c) in text.chars().enumerate() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end_pos = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if end_pos > 0 {
        let json_str = &text[..end_pos];
        serde_json::from_str(json_str).map_err(|e| Error::Json(e))
    } else {
        Err(Error::ExtractionFailed("Invalid JSON structure".to_string()))
    }
}

#[allow(dead_code)]
fn extract_player_url(html: &str) -> Result<String> {
    // Look for player JS URL
    let patterns = [
        r#""jsUrl"\s*:\s*"([^"]+)""#,
        r#""PLAYER_JS_URL"\s*:\s*"([^"]+)""#,
        r#"/s/player/([a-zA-Z0-9_-]+)/player_ias\.vflset/[^"]+\.js"#,
    ];

    for pattern in patterns {
        let re = Regex::new(pattern).unwrap();
        if let Some(caps) = re.captures(html) {
            if let Some(url) = caps.get(1).or(caps.get(0)) {
                let url_str = url.as_str();
                if url_str.starts_with('/') {
                    return Ok(format!("https://www.youtube.com{}", url_str));
                } else if url_str.starts_with("http") {
                    return Ok(url_str.to_string());
                }
            }
        }
    }

    Err(Error::ExtractionFailed("Could not find player URL".to_string()))
}

fn check_playability(player_response: &Value) -> Result<()> {
    if let Some(status) = player_response.get("playabilityStatus") {
        let status_str = status
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("UNKNOWN");

        match status_str {
            "OK" | "LIVE_STREAM_OFFLINE" => Ok(()),
            "LOGIN_REQUIRED" => Err(Error::VideoUnavailable(
                "Video requires login".to_string(),
            )),
            "UNPLAYABLE" | "ERROR" => {
                let reason = status
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("Video is unavailable");
                Err(Error::VideoUnavailable(reason.to_string()))
            }
            _ => Ok(()),
        }
    } else {
        Ok(())
    }
}

fn parse_format(fmt: &Value) -> Option<Format> {
    let itag = fmt.get("itag")?.as_u64()? as u32;
    
    // Get URL - Android client provides direct URLs without cipher
    let url = fmt.get("url").and_then(|u| u.as_str())?.to_string();

    let mime_type = fmt.get("mimeType").and_then(|m| m.as_str()).unwrap_or("");
    let quality = fmt.get("quality").and_then(|q| q.as_str()).unwrap_or("");
    let quality_label = fmt.get("qualityLabel").and_then(|q| q.as_str());
    
    let width = fmt.get("width").and_then(|w| w.as_u64()).map(|w| w as u32);
    let height = fmt.get("height").and_then(|h| h.as_u64()).map(|h| h as u32);
    
    let bitrate = fmt.get("bitrate").and_then(|b| b.as_u64()).map(|b| b as u32);
    let filesize = fmt.get("contentLength")
        .and_then(|c| c.as_str())
        .and_then(|s| s.parse().ok());

    let fps = fmt.get("fps").and_then(|f| f.as_u64()).map(|f| f as u32);
    
    let audio_quality = fmt.get("audioQuality").and_then(|a| a.as_str()).map(|s| s.to_string());
    let audio_sample_rate = fmt.get("audioSampleRate")
        .and_then(|a| a.as_str())
        .and_then(|s| s.parse().ok());
    let audio_channels = fmt.get("audioChannels").and_then(|a| a.as_u64()).map(|a| a as u32);

    // Parse codecs from mimeType
    let (container, video_codec, audio_codec) = parse_mime_type(mime_type);
    
    let is_audio_only = video_codec.is_none() && audio_codec.is_some();
    let is_video_only = video_codec.is_some() && audio_codec.is_none() && width.is_some();

    Some(Format {
        format_id: itag.to_string(),
        url,
        container,
        video_codec,
        audio_codec,
        width,
        height,
        fps,
        bitrate,
        filesize,
        quality: quality.to_string(),
        quality_label: quality_label.map(|s| s.to_string()),
        audio_quality,
        audio_sample_rate,
        audio_channels,
        is_audio_only,
        is_video_only,
    })
}

fn parse_mime_type(mime: &str) -> (String, Option<String>, Option<String>) {
    // e.g., "video/mp4; codecs=\"avc1.42001E, mp4a.40.2\""
    let mut container = "mp4".to_string();
    let mut video_codec = None;
    let mut audio_codec = None;

    if let Some(slash_pos) = mime.find('/') {
        let media_type = &mime[..slash_pos];
        
        // Extract container
        if let Some(semi_pos) = mime.find(';') {
            container = mime[slash_pos + 1..semi_pos].to_string();
        } else {
            container = mime[slash_pos + 1..].to_string();
        }

        // Extract codecs
        if let Some(codecs_start) = mime.find("codecs=\"") {
            let start = codecs_start + 8;
            if let Some(end) = mime[start..].find('"') {
                let codecs = &mime[start..start + end];
                for codec in codecs.split(',').map(|s| s.trim()) {
                    if codec.starts_with("avc") || codec.starts_with("vp") || codec.starts_with("av01") {
                        video_codec = Some(codec.to_string());
                    } else if codec.starts_with("mp4a") || codec.starts_with("opus") || codec.starts_with("vorbis") {
                        audio_codec = Some(codec.to_string());
                    }
                }
            }
        }

        // Infer from media type if no codecs specified
        if media_type == "audio" && audio_codec.is_none() {
            audio_codec = Some("unknown".to_string());
        }
        if media_type == "video" && video_codec.is_none() {
            video_codec = Some("unknown".to_string());
        }
    }

    (container, video_codec, audio_codec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_video_id() {
        // Raw ID
        assert_eq!(parse_video_id("dQw4w9WgXcQ").unwrap(), "dQw4w9WgXcQ");
        
        // Standard watch URL
        assert_eq!(
            parse_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
        
        // Short URL
        assert_eq!(
            parse_video_id("https://youtu.be/dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
        
        // Invalid
        assert!(parse_video_id("invalid").is_err());
    }
}
