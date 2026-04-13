pub mod config;

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const CONFIG_TXT: &str = "/boot/firmware/config.txt";
const CMDLINE_TXT: &str = "/boot/firmware/cmdline.txt";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpuStatus {
    pub driver_loaded: bool,
    pub dtoverlay_set: bool,
    pub gpu_mem_mb: Option<u32>,
    pub firmware_exists: bool,
}

pub fn detect() -> Result<GpuStatus> {
    let modules = fs::read_to_string("/proc/modules").unwrap_or_default();
    let driver_loaded = modules
        .lines()
        .any(|l| l.starts_with("vc4") || l.starts_with("v3d"));

    let config = fs::read_to_string(CONFIG_TXT).unwrap_or_default();
    let dtoverlay_set = config.lines().any(|l| {
        let l = l.trim();
        l.contains("vc4-kms-v3d") && !l.starts_with('#')
    });

    let gpu_mem_mb = config.lines().find_map(|l| {
        let l = l.trim();
        if l.starts_with('#') {
            return None;
        }
        if let Some(val) = l.strip_prefix("gpu_mem=") {
            return val.trim().parse::<u32>().ok();
        }
        if let Some(val) = l.strip_prefix("gpu_mem_1024=") {
            return val.trim().parse::<u32>().ok();
        }
        None
    });

    Ok(GpuStatus {
        driver_loaded,
        dtoverlay_set,
        gpu_mem_mb,
        firmware_exists: Path::new("/opt/vc/lib/libbrcmEGL.so").exists()
            || Path::new("/usr/lib/aarch64-linux-gnu/libEGL.so").exists(),
    })
}

pub fn activate(gpu_mem: u32) -> Result<()> {
    ensure_root()?;
    let config =
        fs::read_to_string(CONFIG_TXT).with_context(|| format!("읽기 실패: {CONFIG_TXT}"))?;

    let mut lines: Vec<String> = config.lines().map(String::from).collect();
    let mut _modified = false;

    if !lines.iter().any(|l| {
        let t = l.trim();
        t.contains("vc4-kms-v3d") && !t.starts_with('#')
    }) {
        if let Some(pos) = lines.iter().position(|l| {
            let t = l.trim();
            t.contains("vc4-kms-v3d") && t.starts_with('#')
        }) {
            lines[pos] = lines[pos].trim_start_matches('#').trim().to_string();
        } else {
            lines.push("dtoverlay=vc4-kms-v3d".to_string());
        }
        _modified = true;
    }

    let mem_line = format!("gpu_mem_1024={gpu_mem}");
    if let Some(pos) = lines.iter().position(|l| {
        let t = l.trim();
        (t.starts_with("gpu_mem=") || t.starts_with("gpu_mem_1024=")) && !t.starts_with('#')
    }) {
        lines[pos] = mem_line;
    } else {
        lines.push(mem_line);
    }
    _modified = true;

    if _modified {
        let content = lines.join("\n");
        fs::write(CONFIG_TXT, content).with_context(|| format!("쓰기 실패: {CONFIG_TXT}"))?;
    }

    Ok(())
}

pub fn deactivate() -> Result<()> {
    ensure_root()?;
    let config =
        fs::read_to_string(CONFIG_TXT).with_context(|| format!("읽기 실패: {CONFIG_TXT}"))?;

    let lines: Vec<String> = config
        .lines()
        .map(|l| {
            let trimmed = l.trim();
            if trimmed.contains("vc4-kms-v3d") && !trimmed.starts_with('#') {
                format!("#{}", l.trim_start_matches('#').trim())
            } else {
                l.to_string()
            }
        })
        .collect();

    let content = lines.join("\n");
    fs::write(CONFIG_TXT, content).with_context(|| format!("쓰기 실패: {CONFIG_TXT}"))?;

    Ok(())
}

pub fn apply_cmdline_tweaks() -> Result<()> {
    ensure_root()?;
    let cmdline =
        fs::read_to_string(CMDLINE_TXT).with_context(|| format!("읽기 실패: {CMDLINE_TXT}"))?;

    let mut parts: Vec<String> = cmdline.split_whitespace().map(String::from).collect();

    let cma = parts.iter().find(|p| p.starts_with("cma="));
    if cma.is_none() {
        parts.push("cma=512M".to_string());
    }

    let content = parts.join(" ");
    fs::write(CMDLINE_TXT, content).with_context(|| format!("쓰기 실패: {CMDLINE_TXT}"))?;

    Ok(())
}

fn ensure_root() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let uid = unsafe { libc::geteuid() };
        if uid != 0 {
            anyhow::bail!("이 작업은 root 권한이 필요합니다. sudo를 사용하세요.");
        }
    }
    Ok(())
}
