use tauri::api::dialog::FileDialogBuilder;
use tauri::Manager;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::io::Write;
use tempfile::NamedTempFile;

#[derive(serde::Serialize)]
struct Video {
    path: String,
    thumbnail: String,
    timestamp: String,
}

#[tauri::command]
async fn scan_videos(folder: String) -> Result<Vec<Video>, String> {
    let entries = fs::read_dir(&folder).map_err(|e| e.to_string())?;
    let mut videos = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("mp4") {
            let thumbnail = generate_thumbnail(&path.to_string_lossy())?;
            let timestamp = extract_timestamp(&path.file_name().unwrap().to_string_lossy());
            videos.push(Video {
                path: path.to_string_lossy().into_owned(),
                thumbnail,
                timestamp,
            });
        }
    }
    Ok(videos)
}

fn extract_timestamp(filename: &str) -> String {
    // Assuming filename format like YYYYMMDD_HHMMSS.mp4
    let re = regex::Regex::new(r"(\d{4})(\d{2})(\d{2})_(\d{2})(\d{2})(\d{2})").unwrap();
    if let Some(caps) = re.captures(filename) {
        format!(
            "{}-{}-{} {}:{}:{}",
            &caps[1], &caps[2], &caps[3], &caps[4], &caps[5], &caps[6]
        )
    } else {
        "Unknown".to_string()
    }
}

fn generate_thumbnail(video_path: &str) -> Result<String, String> {
    let output_file = NamedTempFile::new().map_err(|e| e.to_string())?;
    let output_path = output_file.path().to_string_lossy().into_owned() + ".jpg";
    let status = Command::new("ffmpeg")
        .args([
            "-i", video_path,
            "-vframes", "1",
            "-s", "120x68",
            &output_path,
        ])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(format!("file://{}", output_path))
    } else {
        Err("Thumbnail generation failed".to_string())
    }
}

#[tauri::command]
async fn optimize_video(video_path: String, save_path: String) -> Result<(), String> {
    let output_path = format!("{}/optimized_{}", save_path, Path::new(&video_path).file_name().unwrap().to_string_lossy());
    let status = Command::new("ffmpeg")
        .args([
            "-i", &video_path,
            "-vcodec", "libx264",
            "-s", "1920x1080",
            "-b:v", "5000k",
            "-preset", "fast",
            &output_path,
        ])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("Video optimization failed".to_string())
    }
}

#[tauri::command]
async fn merge_videos(video_paths: Vec<String>, save_path: String) -> Result<(), String> {
    let output_path = format!("{}/merged_{}.mp4", save_path, chrono::Utc::now().timestamp());
    let mut concat_file = NamedTempFile::new().map_err(|e| e.to_string())?;
    for path in &video_paths {
        writeln!(concat_file, "file '{}'", path).map_err(|e| e.to_string())?;
    }
    let concat_file_path = concat_file.path().to_string_lossy().into_owned();
    let status = Command::new("ffmpeg")
        .args([
            "-f", "concat",
            "-safe", "0",
            "-i", &concat_file_path,
            "-c", "copy",
            &output_path,
        ])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        optimize_video(output_path.clone(), save_path).await
    } else {
        Err("Video merging failed".to_string())
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![scan_videos, optimize_video, merge_videos])
        .run(tauri::generate_context!())
        .expect("Error while running Tauri application");
}
