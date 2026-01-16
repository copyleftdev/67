use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

const INNERTUBE_API_KEY: &str = "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w";

#[derive(Debug, Serialize)]
pub struct VideoMetadata {
    pub video_id: String,
    pub title: String,
    pub channel: String,
    pub transcripts: Vec<Transcript>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transcript {
    pub language: String,
    pub language_code: String,
    pub is_auto_generated: bool,
    pub segments: Vec<TranscriptSegment>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start: f64,
    pub duration: f64,
    pub text: String,
}

pub async fn extract_transcripts(
    client: &reqwest::Client,
    video_id: &str,
) -> Result<Vec<Transcript>> {
    let mut transcripts = Vec::new();

    // Call InnerTube player API to get fresh caption track URLs
    let api_url = format!(
        "https://www.youtube.com/youtubei/v1/player?key={}&prettyPrint=false",
        INNERTUBE_API_KEY
    );

    let body = serde_json::json!({
        "context": {
            "client": {
                "hl": "en",
                "clientName": "WEB",
                "clientVersion": "2.20240101.00.00"
            }
        },
        "videoId": video_id
    });

    let response = client
        .post(&api_url)
        .header("Content-Type", "application/json")
        .header("X-Youtube-Client-Name", "1")
        .header("X-Youtube-Client-Version", "2.20240101.00.00")
        .body(body.to_string())
        .send()
        .await?;

    if !response.status().is_success() {
        return Ok(transcripts);
    }

    let player_response: Value = match response.json().await {
        Ok(v) => v,
        Err(_) => return Ok(transcripts),
    };

    // Extract caption tracks
    let caption_tracks = match player_response
        .pointer("/captions/playerCaptionsTracklistRenderer/captionTracks")
        .and_then(|v| v.as_array())
    {
        Some(t) => t,
        None => return Ok(transcripts),
    };

    // Fetch each caption track immediately (URLs expire fast)
    for track in caption_tracks.iter().take(3) {
        let base_url = match track.get("baseUrl").and_then(|v| v.as_str()) {
            Some(url) => url,
            None => continue,
        };

        let language = track
            .pointer("/name/simpleText")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let language_code = track
            .get("languageCode")
            .and_then(|v| v.as_str())
            .unwrap_or("und")
            .to_string();

        let is_auto = track
            .get("kind")
            .and_then(|v| v.as_str())
            .map(|k| k == "asr")
            .unwrap_or(false);

        // Fetch captions in JSON3 format
        let caption_url = format!("{}&fmt=json3", base_url);
        
        let caption_response = match client.get(&caption_url).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };

        let caption_data: Value = match caption_response.json().await {
            Ok(v) => v,
            Err(_) => continue,
        };

        let segments = parse_json3_captions(&caption_data);
        
        if !segments.is_empty() {
            transcripts.push(Transcript {
                language,
                language_code,
                is_auto_generated: is_auto,
                segments,
            });
        }
    }

    Ok(transcripts)
}

fn parse_json3_captions(data: &Value) -> Vec<TranscriptSegment> {
    let mut segments = Vec::new();

    let events = match data.get("events").and_then(|v| v.as_array()) {
        Some(e) => e,
        None => return segments,
    };

    for event in events {
        let segs = match event.get("segs").and_then(|v| v.as_array()) {
            Some(s) => s,
            None => continue,
        };

        let start_ms = event.get("tStartMs").and_then(|v| v.as_u64()).unwrap_or(0);
        let duration_ms = event.get("dDurationMs").and_then(|v| v.as_u64()).unwrap_or(0);

        let text: String = segs
            .iter()
            .filter_map(|seg| seg.get("utf8").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("")
            .replace('\n', " ")
            .trim()
            .to_string();

        if !text.is_empty() {
            segments.push(TranscriptSegment {
                start: start_ms as f64 / 1000.0,
                duration: duration_ms as f64 / 1000.0,
                text,
            });
        }
    }

    segments
}
