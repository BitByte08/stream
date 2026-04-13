use anyhow::{Context, Result};
use std::process::Command;

const REQUIRED_PACKAGES: &[&str] = &[
    "mpv",
    "ffmpeg",
    "libraspberrypi0",
    "libdrm2",
    "mesa-utils",
    "firmware-linux-nonfree",
];

const OPTIONAL_PACKAGES: &[&str] = &["v4l-utils", "libva2", "i965-va-driver"];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DepStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
}

pub fn check_dependencies() -> Result<Vec<DepStatus>> {
    let mut statuses = Vec::new();
    let all_pkgs: Vec<&str> = REQUIRED_PACKAGES
        .iter()
        .chain(OPTIONAL_PACKAGES.iter())
        .copied()
        .collect();

    for pkg in &all_pkgs {
        let output = Command::new("dpkg-query")
            .args(["-W", "-f", "${Version}\\n", pkg])
            .output();

        let (installed, version) = match output {
            Ok(o) if o.status.success() => {
                let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let ver = if ver.is_empty() { None } else { Some(ver) };
                (true, ver)
            }
            _ => (false, None),
        };

        statuses.push(DepStatus {
            name: pkg.to_string(),
            installed,
            version,
        });
    }

    Ok(statuses)
}

pub fn install_missing() -> Result<()> {
    let statuses = check_dependencies()?;

    let missing: Vec<&str> = statuses
        .iter()
        .filter(|s| !s.installed && REQUIRED_PACKAGES.contains(&s.name.as_str()))
        .map(|s| s.name.as_str())
        .collect();

    if missing.is_empty() {
        println!("모든 필수 종속성이 설치되어 있습니다.");
        return Ok(());
    }

    println!("설치할 종속성: {}", missing.join(", "));

    let mut args = vec!["apt", "install", "-y"];
    args.extend(missing.iter().copied());

    let status = Command::new("sudo")
        .args(&args)
        .status()
        .context("apt install 실행 실패")?;

    if !status.success() {
        anyhow::bail!("apt install 실패");
    }

    println!("종속성 설치 완료.");
    Ok(())
}

pub fn install_all() -> Result<()> {
    let mut args = vec!["apt", "install", "-y"];
    args.extend(REQUIRED_PACKAGES.iter().copied());
    args.extend(OPTIONAL_PACKAGES.iter().copied());

    let status = Command::new("sudo")
        .args(&args)
        .status()
        .context("apt install 실행 실패")?;

    if !status.success() {
        anyhow::bail!("apt install 실패");
    }

    println!("모든 종속성 설치 완료.");
    Ok(())
}

pub fn required_packages() -> Vec<&'static str> {
    REQUIRED_PACKAGES.to_vec()
}

pub fn optional_packages() -> Vec<&'static str> {
    OPTIONAL_PACKAGES.to_vec()
}
