use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use url::Url;

#[derive(Debug, Clone)]
pub struct AudioFile {
    pub path: PathBuf,
    pub duration_secs: Option<u32>,
    pub title: String,
}

/// Validate that a URL is a proper http/https URL
pub fn validate_url(raw: &str) -> Result<Url> {
    if raw.is_empty() {
        bail!("URL must not be empty");
    }

    let parsed = Url::parse(raw).context("Failed to parse URL")?;

    match parsed.scheme() {
        "http" | "https" => Ok(parsed),
        scheme => bail!("Unsupported URL scheme '{}'; only http and https are allowed", scheme),
    }
}

/// Download video and extract audio using yt-dlp + ffmpeg
pub fn download_and_extract(url: &str, output_dir: &Path) -> Result<AudioFile> {
    let validated = validate_url(url)?;

    // Ensure output directory exists
    std::fs::create_dir_all(output_dir)
        .context("Failed to create output directory")?;

    // Use yt-dlp to download and extract audio
    let output_template = output_dir.join("%(title)s.%(ext)s");
    let output_template_str = output_template.to_string_lossy();

    let status = Command::new("yt-dlp")
        .args([
            "-x",
            "--audio-format", "mp3",
            "--audio-quality", "0",
            "--print", "after_move:filepath",
            "-o", output_template_str.as_ref(),
            validated.as_str(),
        ])
        .output();

    let output = match status {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!("yt-dlp not found; please install yt-dlp to download videos")
        }
        Err(e) => return Err(e).context("Failed to run yt-dlp"),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("yt-dlp failed: {}", stderr);
    }

    // The printed filepath is the downloaded mp3
    let filepath_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let audio_path = PathBuf::from(&filepath_str);

    if !audio_path.exists() {
        bail!("yt-dlp reported file '{}' but it does not exist", audio_path.display());
    }

    // Extract title from filename (stem)
    let title = audio_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Title")
        .to_string();

    // Get duration via ffprobe
    let duration_secs = probe_duration(&audio_path);

    Ok(AudioFile {
        path: audio_path,
        duration_secs,
        title,
    })
}

#[derive(Deserialize)]
struct FfprobeOutput {
    streams: Vec<FfprobeStream>,
}

#[derive(Deserialize)]
struct FfprobeStream {
    duration: Option<String>,
}

/// Use ffprobe to get audio duration in seconds
pub fn probe_duration(path: &Path) -> Option<u32> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_streams",
            path.to_string_lossy().as_ref(),
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout).ok()?;

    parsed.streams.into_iter().find_map(|s| {
        s.duration
            .as_deref()
            .and_then(|d| d.parse::<f64>().ok())
            .map(|d| d as u32)
    })
}
