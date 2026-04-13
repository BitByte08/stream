pub mod params;

use anyhow::{Context, Result};
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamStatus {
    pub mpv_installed: bool,
    pub mpv_version: Option<String>,
    pub ffmpeg_installed: bool,
}

pub fn detect_environment() -> Result<StreamStatus> {
    let mpv_output = Command::new("mpv").arg("--version").output();

    let (mpv_installed, mpv_version) = match mpv_output {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .map(|s| s.to_string());
            (true, ver)
        }
        _ => (false, None),
    };

    let ffmpeg_installed = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    Ok(StreamStatus {
        mpv_installed,
        mpv_version,
        ffmpeg_installed,
    })
}

pub fn play(source: &str, extra_params: &[String]) -> Result<()> {
    let config = params::StreamParams::new_4k60();
    let mut args = config.to_args();
    args.push(source.to_string());
    args.extend(extra_params.iter().cloned());

    let status = Command::new("mpv")
        .args(&args)
        .status()
        .context("mpv 실행 실패. mpv가 설치되어 있는지 확인하세요.")?;

    if !status.success() {
        anyhow::bail!("mpv가 오류 코드로 종료됨: {:?}", status.code());
    }
    Ok(())
}

pub fn play_with_profile(source: &str, profile: &str, extra_params: &[String]) -> Result<()> {
    let config = match profile {
        "4k60" | "default" => params::StreamParams::new_4k60(),
        "4k30" => params::StreamParams::new_4k30(),
        "1080p60" => params::StreamParams::new_1080p60(),
        "low-latency" => params::StreamParams::new_low_latency(),
        _ => anyhow::bail!(
            "알 수 없는 프로필: {profile}. 사용 가능: 4k60, 4k30, 1080p60, low-latency"
        ),
    };

    let needs_transcode = should_transcode(source)?;

    if needs_transcode {
        play_via_transcode_pipe(source, &config, extra_params)
    } else {
        play_direct(source, &config, extra_params)
    }
}

fn should_transcode(source: &str) -> Result<bool> {
    if source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("rtsp://")
        || source.starts_with("rtp://")
    {
        return Ok(false);
    }

    if !std::path::Path::new(source).exists() {
        return Ok(false);
    }

    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name",
            "-of",
            "csv=p=0",
            source,
        ])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let codec = String::from_utf8_lossy(&o.stdout).trim().to_string();
            Ok(codec == "h264")
        }
        _ => Ok(false),
    }
}

fn play_direct(source: &str, config: &params::StreamParams, extra_params: &[String]) -> Result<()> {
    let mut args = config.to_args();
    args.push(source.to_string());
    args.extend(extra_params.iter().cloned());

    let status = Command::new("mpv")
        .args(&args)
        .status()
        .context("mpv 실행 실패")?;

    if !status.success() {
        anyhow::bail!("mpv가 오류 코드로 종료됨: {:?}", status.code());
    }
    Ok(())
}

fn play_via_transcode_pipe(
    source: &str,
    config: &params::StreamParams,
    extra_params: &[String],
) -> Result<()> {
    println!("H.264 감지 → H.265 실시간 변환 재생 (파이프)");

    let resolution = match (config.width, config.height) {
        (3840, 2160) => "3840:2160",
        (1920, 1080) => "1920:1080",
        _ => "3840:2160",
    };

    let ffmpeg_args = vec![
        "-i".to_string(),
        source.to_string(),
        "-c:v".to_string(),
        "hevc_v4l2m2m".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-vf".to_string(),
        format!("scale={}", resolution),
        "-c:a".to_string(),
        "copy".to_string(),
        "-f".to_string(),
        "matroska".to_string(),
        "-".to_string(),
    ];

    let ffmpeg = Command::new("ffmpeg")
        .args(&ffmpeg_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("ffmpeg 실행 실패")?;

    let ffmpeg_stdout = ffmpeg.stdout.context("ffmpeg stdout 파이프 실패")?;

    let mut mpv_args = config.to_args();
    mpv_args.push("-".to_string());
    mpv_args.extend(extra_params.iter().cloned());

    let mpv_status = Command::new("mpv")
        .args(&mpv_args)
        .stdin(ffmpeg_stdout)
        .status()
        .context("mpv 실행 실패")?;

    if !mpv_status.success() {
        println!("HEVC HW 인코딩 실패, 소프트웨어 폴백...");
        play_via_transcode_pipe_sw(source, config, extra_params)?;
    }

    Ok(())
}

fn play_via_transcode_pipe_sw(
    source: &str,
    config: &params::StreamParams,
    extra_params: &[String],
) -> Result<()> {
    let resolution = match (config.width, config.height) {
        (3840, 2160) => "3840:2160",
        (1920, 1080) => "1920:1080",
        _ => "3840:2160",
    };

    let ffmpeg_args = vec![
        "-i".to_string(),
        source.to_string(),
        "-c:v".to_string(),
        "libx265".to_string(),
        "-preset".to_string(),
        "ultrafast".to_string(),
        "-crf".to_string(),
        "23".to_string(),
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        "-vf".to_string(),
        format!("scale={}", resolution),
        "-c:a".to_string(),
        "copy".to_string(),
        "-f".to_string(),
        "matroska".to_string(),
        "-".to_string(),
    ];

    let ffmpeg = Command::new("ffmpeg")
        .args(&ffmpeg_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("ffmpeg SW 실행 실패")?;

    let ffmpeg_stdout = ffmpeg.stdout.context("ffmpeg stdout 파이프 실패")?;

    let mut mpv_args = config.to_args();
    mpv_args.push("-".to_string());
    mpv_args.extend(extra_params.iter().cloned());

    let status = Command::new("mpv")
        .args(&mpv_args)
        .stdin(ffmpeg_stdout)
        .status()
        .context("mpv 실행 실패")?;

    if !status.success() {
        anyhow::bail!("mpv 오류: {:?}", status.code());
    }
    Ok(())
}
