use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranscodeConfig {
    pub input: PathBuf,
    pub output: PathBuf,
    pub codec: String,
    pub preset: String,
    pub crf: u8,
    pub pixel_format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub hw_accel: bool,
    pub copy_audio: bool,
    pub container: String,
}

impl Default for TranscodeConfig {
    fn default() -> Self {
        Self {
            input: PathBuf::new(),
            output: PathBuf::new(),
            codec: "hevc_v4l2m2m".to_string(),
            preset: "medium".to_string(),
            crf: 23,
            pixel_format: "yuv420p".to_string(),
            width: None,
            height: None,
            hw_accel: true,
            copy_audio: true,
            container: "mp4".to_string(),
        }
    }
}

impl TranscodeConfig {
    pub fn new_h264_to_h265(input: &str, output: Option<&str>) -> Self {
        let input_path = PathBuf::from(input);
        let output_path = match output {
            Some(o) => PathBuf::from(o),
            None => {
                let stem = input_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("output");
                let parent = input_path.parent().unwrap_or(Path::new("."));
                parent.join(format!("{}_h265.{}", stem, "mp4"))
            }
        };

        Self {
            input: input_path,
            output: output_path,
            ..Default::default()
        }
    }

    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    pub fn with_crf(mut self, crf: u8) -> Self {
        self.crf = crf.min(51);
        self
    }

    pub fn with_software_fallback(mut self) -> Self {
        self.hw_accel = false;
        self.codec = "libx265".to_string();
        self
    }

    pub fn to_ffmpeg_args(&self) -> Vec<String> {
        let mut args = vec![
            "-y".to_string(),
            "-i".to_string(),
            self.input.to_string_lossy().to_string(),
        ];

        if self.hw_accel {
            args.push("-c:v".to_string());
            args.push(self.codec.clone());
        } else {
            args.push("-c:v".to_string());
            args.push(self.codec.clone());
            args.push("-preset".to_string());
            args.push(self.preset.clone());
        }

        if self.codec == "libx265" {
            args.push("-crf".to_string());
            args.push(self.crf.to_string());
        }

        args.push("-pix_fmt".to_string());
        args.push(self.pixel_format.clone());

        if let (Some(w), Some(h)) = (self.width, self.height) {
            args.push("-vf".to_string());
            args.push(format!("scale={}:{}", w, h));
        }

        if self.copy_audio {
            args.push("-c:a".to_string());
            args.push("copy".to_string());
        } else {
            args.push("-c:a".to_string());
            args.push("aac".to_string());
            args.push("-b:a".to_string());
            args.push("128k".to_string());
        }

        args.push("-movflags".to_string());
        args.push("+faststart".to_string());

        args.push(self.output.to_string_lossy().to_string());
        args
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProbeResult {
    pub path: String,
    pub duration_secs: Option<f64>,
    pub video_codec: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub audio_codec: Option<String>,
    pub size_bytes: u64,
    pub needs_transcode: bool,
}

pub fn probe(input: &str) -> Result<ProbeResult> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            input,
        ])
        .output()
        .context("ffprobe 실행 실패. ffmpeg가 설치되어 있는지 확인하세요.")?;

    if !output.status.success() {
        anyhow::bail!("ffprobe 실패: {}", String::from_utf8_lossy(&output.stderr));
    }

    let probe_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("ffprobe 출력 파싱 실패")?;

    let mut video_codec = None;
    let mut width = None;
    let mut height = None;
    let mut fps = None;
    let mut audio_codec = None;

    if let Some(streams) = probe_json["streams"].as_array() {
        for stream in streams {
            let codec_type = stream["codec_type"].as_str().unwrap_or("");
            match codec_type {
                "video" => {
                    video_codec = stream["codec_name"].as_str().map(String::from);
                    width = stream["width"].as_u64().map(|v| v as u32);
                    height = stream["height"].as_u64().map(|v| v as u32);
                    if let Some(r_framerate) = stream["r_frame_rate"].as_str() {
                        let parts: Vec<&str> = r_framerate.split('/').collect();
                        if parts.len() == 2 {
                            if let (Ok(num), Ok(den)) =
                                (parts[0].parse::<f64>(), parts[1].parse::<f64>())
                            {
                                if den > 0.0 {
                                    fps = Some(num / den);
                                }
                            }
                        }
                    }
                }
                "audio" => {
                    audio_codec = stream["codec_name"].as_str().map(String::from);
                }
                _ => {}
            }
        }
    }

    let duration_secs = probe_json["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok());

    let size_bytes = probe_json["format"]["size"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let needs_transcode = video_codec.as_deref() == Some("h264");

    Ok(ProbeResult {
        path: input.to_string(),
        duration_secs,
        video_codec,
        width,
        height,
        fps,
        audio_codec,
        size_bytes,
        needs_transcode,
    })
}

pub fn transcode(config: &TranscodeConfig) -> Result<()> {
    if !config.input.exists() {
        anyhow::bail!("입력 파일 없음: {}", config.input.display());
    }

    let probe_result = probe(config.input.to_string_lossy().as_ref())?;

    println!("\n=== 트랜스코드 정보 ===\n");
    println!("  입력: {}", config.input.display());
    if let Some(codec) = &probe_result.video_codec {
        println!("  비디오 코덱: {}", codec);
    }
    if let (Some(w), Some(h)) = (probe_result.width, probe_result.height) {
        println!("  해상도: {}x{}", w, h);
    }
    if let Some(fps) = probe_result.fps {
        println!("  FPS: {:.2}", fps);
    }
    println!("  출력: {}", config.output.display());
    println!("  출력 코덱: {}", config.codec);
    println!("  HW 가속: {}", if config.hw_accel { "ON" } else { "OFF" });

    if !probe_result.needs_transcode {
        println!("\n  이미 H.265 이거나 비디오 코덱이 아닙니다.");
        if probe_result.video_codec.as_deref() == Some("hevc") {
            println!("  변환 불필요: HEVC(H.265)입니다.");
        }
    }

    let args = config.to_ffmpeg_args();
    println!("\n  ffmpeg {}", args.join(" "));
    println!();

    let status = Command::new("ffmpeg")
        .args(&args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context("ffmpeg 실행 실패")?;

    if !status.success() {
        anyhow::bail!("ffmpeg 오류: {:?}", status.code());
    }

    println!("\n✅ 변환 완료: {}", config.output.display());
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchResult {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

pub fn transcode_batch(directory: &str, recursive: bool, crf: u8) -> Result<BatchResult> {
    let dir = Path::new(directory);
    if !dir.is_dir() {
        anyhow::bail!("디렉토리 아님: {}", directory);
    }

    let files = collect_video_files(dir, recursive)?;

    if files.is_empty() {
        println!("비디오 파일 없음: {}", directory);
        return Ok(BatchResult {
            total: 0,
            success: 0,
            failed: 0,
            skipped: 0,
            errors: vec![],
        });
    }

    println!("\n=== 배치 트랜스코드 ===\n");
    println!("  대상: {}개 파일\n", files.len());

    let mut result = BatchResult {
        total: files.len(),
        success: 0,
        failed: 0,
        skipped: 0,
        errors: vec![],
    };

    for (i, file) in files.iter().enumerate() {
        println!("[{}/{}] {}", i + 1, files.len(), file.display());

        let probe_result = match probe(&file.to_string_lossy()) {
            Ok(p) => p,
            Err(e) => {
                result.failed += 1;
                result
                    .errors
                    .push(format!("{}: probe 실패 - {}", file.display(), e));
                continue;
            }
        };

        if !probe_result.needs_transcode {
            println!(
                "  건너뜀 (이미 {})",
                probe_result.video_codec.unwrap_or_default()
            );
            result.skipped += 1;
            continue;
        }

        let config = TranscodeConfig::new_h264_to_h265(&file.to_string_lossy(), None).with_crf(crf);

        match transcode(&config) {
            Ok(_) => result.success += 1,
            Err(e) => {
                result.failed += 1;
                result.errors.push(format!("{}: {}", file.display(), e));
            }
        }
    }

    println!("\n=== 결과 ===");
    println!(
        "  성공: {} | 실패: {} | 건너뜀: {} / 총 {}",
        result.success, result.failed, result.skipped, result.total
    );

    Ok(result)
}

fn collect_video_files(dir: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let extensions = [
        "mp4", "mkv", "avi", "mov", "flv", "wmv", "webm", "ts", "mts", "m2ts",
    ];
    let mut files = Vec::new();

    let entries = fs::read_dir(dir).context("디렉토리 읽기 실패")?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() && recursive {
            files.extend(collect_video_files(&path, true)?);
        } else if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.contains(&ext.to_lowercase().as_str()) {
                    files.push(path);
                }
            }
        }
    }

    Ok(files)
}

pub fn watch_and_transcode(directory: &str, crf: u8) -> Result<()> {
    println!("디렉토리 감시 중: {}", directory);
    println!("새 H.264 파일이 추가되면 자동 변환합니다. Ctrl+C로 종료.\n");

    let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();

    let dir = Path::new(directory);
    if !dir.is_dir() {
        anyhow::bail!("디렉토리 아님: {}", directory);
    }

    loop {
        if let Ok(files) = collect_video_files(dir, false) {
            for file in files {
                let key = file.to_string_lossy().to_string();
                let output_key = key
                    .replace(".mp4", "_h265.mp4")
                    .replace(".mkv", "_h265.mp4")
                    .replace(".avi", "_h265.mp4");

                if processed.contains(&key) || Path::new(&output_key).exists() {
                    continue;
                }

                println!("새 파일 감지: {}", key);

                match probe(&key) {
                    Ok(p) if p.needs_transcode => {
                        let config = TranscodeConfig::new_h264_to_h265(&key, None).with_crf(crf);
                        match transcode(&config) {
                            Ok(_) => println!("✅ 자동 변환 완료: {}", key),
                            Err(e) => eprintln!("❌ 변환 실패: {}", e),
                        }
                    }
                    Ok(_) => println!("  건너뜀 (H.264 아님)"),
                    Err(e) => eprintln!("  probe 실패: {}", e),
                }

                processed.insert(key);
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
