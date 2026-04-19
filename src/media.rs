use anyhow::{bail, Context, Result};
use regex::Regex;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use url::Url;

// ── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AudioFile {
    pub path: PathBuf,
    pub duration_secs: Option<u32>,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct VimeoStream {
    pub url: String,
    pub title: String,
}

// ── URL validation ────────────────────────────────────────────────────────────

pub fn validate_url(raw: &str) -> Result<Url> {
    if raw.is_empty() {
        bail!("URL must not be empty");
    }
    let parsed = Url::parse(raw).context("Failed to parse URL")?;
    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        s => bail!("Unsupported scheme '{}'; only http/https allowed", s),
    }
}

// ── Vimeo page parsing ────────────────────────────────────────────────────────

/// Scan `html` for Vimeo player iframes (including lazy-loaded `data-src`) and
/// return the numeric video ID of the first one found.
pub fn extract_vimeo_id_from_page(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let sel = Selector::parse("iframe").ok()?;

    for el in document.select(&sel) {
        let attrs = el.value();
        for attr in ["src", "data-src"] {
            if let Some(src) = attrs.attr(attr) {
                if let Some(id) = vimeo_id_from_url(src) {
                    return Some(id);
                }
            }
        }
    }
    None
}

fn vimeo_id_from_url(url: &str) -> Option<String> {
    let re =
        Regex::new(r"(?:vimeo\.com/(?:video/)?|player\.vimeo\.com/video/)(\d+)").ok()?;
    re.captures(url).map(|c| c[1].to_string())
}

// ── Vimeo config JSON parsing ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct VimeoConfigRoot {
    video: VimeoVideoMeta,
    request: VimeoRequest,
}

#[derive(Deserialize)]
struct VimeoVideoMeta {
    title: Option<String>,
}

#[derive(Deserialize)]
struct VimeoRequest {
    files: VimeoFiles,
}

#[derive(Deserialize)]
struct VimeoFiles {
    progressive: Option<Vec<VimeoProgressive>>,
}

#[derive(Deserialize)]
struct VimeoProgressive {
    url: String,
    height: Option<u32>,
}

/// Parse the JSON returned by `player.vimeo.com/video/{id}/config` and
/// return the highest-quality progressive stream + title.
pub fn parse_vimeo_config(json: &str) -> Result<VimeoStream> {
    let root: VimeoConfigRoot =
        serde_json::from_str(json).context("Invalid Vimeo config JSON")?;

    let streams = root
        .request
        .files
        .progressive
        .filter(|v| !v.is_empty())
        .context("No progressive streams found in Vimeo config")?;

    let best = streams
        .into_iter()
        .max_by_key(|s| s.height.unwrap_or(0))
        .context("Empty progressive stream list")?;

    Ok(VimeoStream {
        url: best.url,
        title: root.video.title.unwrap_or_else(|| "Untitled".to_string()),
    })
}

// ── Network helpers ───────────────────────────────────────────────────────────

async fn http_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (compatible; to_podcast/1.0)")
        .build()?)
}

async fn fetch_html(url: &str) -> Result<String> {
    let client = http_client().await?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("HTTP GET failed")?;
    let resp = resp
        .error_for_status()
        .context("HTTP error response")?;
    resp.text().await.context("Failed to read response body")
}

async fn fetch_vimeo_stream(video_id: &str) -> Result<VimeoStream> {
    let config_url = format!("https://player.vimeo.com/video/{}/config", video_id);
    let client = http_client().await?;
    let resp = client
        .get(&config_url)
        .header("Referer", "https://vimeo.com/")
        .send()
        .await
        .context("Vimeo config request failed")?;
    let resp = resp
        .error_for_status()
        .context("Vimeo config HTTP error")?;
    let json = resp.text().await.context("Failed to read Vimeo config")?;
    parse_vimeo_config(&json)
}

// ── Audio extraction ──────────────────────────────────────────────────────────

fn safe_filename(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    s.trim_matches('_').to_string()
}

async fn run_ffmpeg(stream_url: &str, title: &str, output_dir: &Path) -> Result<PathBuf> {
    tokio::fs::create_dir_all(output_dir)
        .await
        .context("Failed to create output directory")?;

    let stem = safe_filename(title);
    let out_path = output_dir.join(format!("{}.mp3", stem));

    let result = Command::new("ffmpeg")
        .args([
            "-y",
            "-i", stream_url,
            "-vn",
            "-acodec", "libmp3lame",
            "-q:a", "2",
            out_path.to_string_lossy().as_ref(),
        ])
        .output()
        .await;

    let output = match result {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!("ffmpeg not found; please install ffmpeg")
        }
        Err(e) => return Err(e).context("Failed to spawn ffmpeg"),
        Ok(o) => o,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ffmpeg failed: {}", stderr);
    }

    Ok(out_path)
}

async fn probe_duration(path: &Path) -> Option<u32> {
    let out = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_streams",
            path.to_string_lossy().as_ref(),
        ])
        .output()
        .await
        .ok()?;

    #[derive(Deserialize)]
    struct Probe { streams: Vec<Stream> }
    #[derive(Deserialize)]
    struct Stream { duration: Option<String> }

    let probe: Probe = serde_json::from_slice(&out.stdout).ok()?;
    probe.streams.into_iter().find_map(|s| {
        s.duration?.parse::<f64>().ok().map(|d| d as u32)
    })
}

// ── Public async entry point ──────────────────────────────────────────────────

/// Fetch `page_url`, locate the embedded Vimeo video, extract its audio to
/// `output_dir`, and return an `AudioFile` with the mp3 path and metadata.
pub async fn process_url(page_url: &str, output_dir: &Path) -> Result<AudioFile> {
    validate_url(page_url)?;

    let html = fetch_html(page_url).await?;
    let video_id = extract_vimeo_id_from_page(&html)
        .context("No Vimeo embed found on that page")?;

    let stream = fetch_vimeo_stream(&video_id).await?;
    let mp3_path = run_ffmpeg(&stream.url, &stream.title, output_dir).await?;
    let duration = probe_duration(&mp3_path).await;

    Ok(AudioFile {
        path: mp3_path,
        duration_secs: duration,
        title: stream.title,
    })
}
