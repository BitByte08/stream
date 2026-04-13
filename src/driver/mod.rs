pub mod blacklist;

use anyhow::{Context, Result};
use std::fs;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModuleInfo {
    pub name: String,
    pub size: u64,
    pub used_count: u32,
    pub used_by: Vec<String>,
}

pub fn list_loaded() -> Result<Vec<ModuleInfo>> {
    let content = fs::read_to_string("/proc/modules").context("/proc/modules 읽기 실패")?;

    let mut modules = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.splitn(4, ' ').collect();
        if parts.len() < 2 {
            continue;
        }

        let name = parts[0].replace('_', "-");
        let size: u64 = parts[1].parse().unwrap_or(0);
        let used_count: u32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

        let used_by = if parts.len() > 3 {
            let deps = parts[3].split('-').next().unwrap_or("");
            deps.split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.trim().replace('_', "-"))
                .collect()
        } else {
            vec![]
        };

        modules.push(ModuleInfo {
            name,
            size,
            used_count,
            used_by,
        });
    }
    Ok(modules)
}

pub fn load_module(module: &str) -> Result<()> {
    let status = std::process::Command::new("modprobe")
        .arg(module)
        .status()
        .with_context(|| format!("modprobe {module} 실행 실패"))?;
    if !status.success() {
        anyhow::bail!("modprobe {} 실패", module);
    }
    Ok(())
}

pub fn unload_module(module: &str) -> Result<()> {
    let status = std::process::Command::new("modprobe")
        .args(["-r", module])
        .status()
        .with_context(|| format!("modprobe -r {module} 실행 실패"))?;
    if !status.success() {
        anyhow::bail!("modprobe -r {} 실패", module);
    }
    Ok(())
}
