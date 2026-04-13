use anyhow::{Context, Result};
use std::fs;
use std::io::{self, BufRead, Write};

const CONF_PATH: &str = "/etc/modprobe.d/stream-cli-blacklist.conf";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlacklistEntry {
    pub module: String,
    pub size: u64,
    pub used_count: u32,
    pub used_by: Vec<String>,
    pub recommended: bool,
    pub currently_blacklisted: bool,
    pub category: String,
    pub reason: String,
}

const RECOMMENDED_MODULES: &[&str] = &[
    "bluetooth",
    "btusb",
    "hci_uart",
    "snd_bcm2835",
    "w1_gpio",
    "w1_therm",
    "snd_usb_audio",
];

fn module_category(name: &str) -> (String, String) {
    match name {
        "brcmfmac" | "brcmutil" | "cfg80211" => ("WiFi".into(), "WiFi 드라이버".into()),
        "bluetooth" | "btusb" | "hci_uart" | "btbcm" | "btintel" | "btmtk" => {
            ("Bluetooth".into(), "Bluetooth 드라이버".into())
        }
        "rfkill" => ("RF".into(), "RF 제어 모듈".into()),
        "snd_bcm2835" | "snd_usb_audio" | "snd_soc_core" | "snd_pcm" | "snd_timer" | "snd" => {
            ("Audio".into(), "오디오 드라이버".into())
        }
        "w1_gpio" | "w1_therm" | "wire" => ("1-Wire".into(), "1-Wire 센서 버스".into()),
        "i2c-bcm2835" | "i2c_dev" | "i2c_bcm2835" => ("I2C".into(), "I2C 버스".into()),
        "spi_bcm2835" | "spi_bcm2835aux" => ("SPI".into(), "SPI 버스".into()),
        "vc4" | "v3d" | "drm" | "drm_kms_helper" => {
            ("GPU/DRM".into(), "GPU/디스플레이 (비활성화 주의)".into())
        }
        "uvcvideo" | "videodev" => ("Camera".into(), "카메라/비디오".into()),
        "usbhid" | "hid_generic" | "hid" => ("HID".into(), "USB 입력 장치 (비활성화 주의)".into()),
        "sdhci" | "sdhci_pltfm" | "sdhci_iproc" => {
            ("SD".into(), "SD카드 컨트롤러 (비활성화 주의)".into())
        }
        "lan78xx" => ("Ethernet".into(), "USB 이더넷 (비활성화 주의)".into()),
        "ipv6" => ("Network".into(), "IPv6 프로토콜".into()),
        _ => ("Other".into(), String::new()),
    }
}

pub fn scan_loaded_modules() -> Result<Vec<BlacklistEntry>> {
    let content = fs::read_to_string("/proc/modules").context("/proc/modules 읽기 실패")?;

    let blacklisted = read_blacklist_modules();

    let mut entries: Vec<BlacklistEntry> = content
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                return None;
            }

            let name = parts[0].replace('_', "-");
            let size: u64 = parts[1].parse().unwrap_or(0);
            let used_count: u32 = parts[2].parse().unwrap_or(0);

            let used_by = if parts.len() > 3 && parts[3] != "-" {
                parts[3]
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim().replace('_', "-"))
                    .collect()
            } else {
                vec![]
            };

            let (category, reason) = module_category(&name);

            Some(BlacklistEntry {
                recommended: RECOMMENDED_MODULES.contains(&name.as_str()),
                currently_blacklisted: blacklisted.contains(&name),
                module: name,
                size,
                used_count,
                used_by,
                category,
                reason,
            })
        })
        .collect();

    entries.sort_by(|a, b| {
        b.recommended
            .cmp(&a.recommended)
            .then(a.category.cmp(&b.category))
            .then(a.module.cmp(&b.module))
    });

    Ok(entries)
}

pub fn interactive_select(entries: &[BlacklistEntry]) -> Result<Vec<String>> {
    if entries.is_empty() {
        println!("로드된 커널 모듈이 없습니다.");
        return Ok(vec![]);
    }

    let mut selected: Vec<bool> = entries.iter().map(|e| e.currently_blacklisted).collect();

    loop {
        println!("\n╔══════════════════════════════════════════════════════════════════╗");
        println!("║              커널 모듈 블랙리스트 관리                          ║");
        println!("╚══════════════════════════════════════════════════════════════════╝");
        println!("  [■] 선택됨  [□] 선택안됨  ★ 권장");
        println!("  명령: 번호(1 3 5), 범위(1-5), r(권장), a(전체), n(해제), d(적용), q(종료)\n");

        println!(
            "{:<4} {:<3} {:<5} {:<20} {:<10} {:>8} {:>5}  {}",
            "", "", "선택", "모듈명", "카테고리", "크기", "사용", "의존 / 비고"
        );
        println!("{}", "─".repeat(90));

        for (i, entry) in entries.iter().enumerate() {
            let check = if selected[i] { "■" } else { "□" };
            let rec = if entry.recommended { "★" } else { " " };
            let cur = if entry.currently_blacklisted {
                "🔒"
            } else {
                " "
            };
            let deps = if entry.used_by.is_empty() {
                "-".into()
            } else if entry.used_by.len() <= 3 {
                entry.used_by.join(",")
            } else {
                format!(
                    "{}, ...(총{}개)",
                    entry.used_by[..3].join(","),
                    entry.used_by.len()
                )
            };
            let warning = match entry.category.as_str() {
                "GPU/DRM" | "HID" | "SD" | "Ethernet" => " ⚠",
                _ => "",
            };
            println!(
                "{:<3} {} {:1}{:<1}{:<1} {:<20} {:<10} {:>7}K {:>4}  {}{}",
                i + 1,
                check,
                rec,
                cur,
                "",
                entry.module,
                entry.category,
                entry.size / 1024,
                entry.used_count,
                if entry.reason.is_empty() {
                    deps
                } else {
                    entry.reason.clone()
                },
                warning,
            );
        }

        print!("\n명령 > ");
        io::stdout().flush()?;

        let stdin = io::stdin();
        let input = stdin
            .lock()
            .lines()
            .next()
            .context("입력 오류")?
            .context("입력 오류")?
            .trim()
            .to_lowercase();

        match input.as_str() {
            "q" => {
                println!("취소됨.");
                return Ok(vec![]);
            }
            "d" => break,
            "r" => {
                for (i, entry) in entries.iter().enumerate() {
                    if entry.recommended {
                        selected[i] = true;
                    } else if !entry.currently_blacklisted {
                        selected[i] = false;
                    }
                }
            }
            "a" => selected.iter_mut().for_each(|s| *s = true),
            "n" => selected.iter_mut().for_each(|s| *s = false),
            _ => {
                if let Ok(indices) = parse_selection(&input, entries.len()) {
                    for idx in indices {
                        selected[idx] = !selected[idx];
                    }
                }
            }
        }
    }

    let chosen: Vec<String> = entries
        .iter()
        .enumerate()
        .filter(|(i, _)| selected[*i])
        .map(|(_, e)| e.module.clone())
        .collect();

    println!("\n최종 선택:");
    for name in &chosen {
        let entry = entries.iter().find(|e| &e.module == name);
        let warning = entry
            .map(|e| match e.category.as_str() {
                "GPU/DRM" | "HID" | "SD" | "Ethernet" => " ⚠ 주의: 시스템에 영향을 줄 수 있습니다",
                _ => "",
            })
            .unwrap_or("");
        println!("  ■ {}{}", name, warning);
    }

    Ok(chosen)
}

fn parse_selection(input: &str, max: usize) -> Result<Vec<usize>> {
    let mut indices = Vec::new();
    for part in input.split_whitespace() {
        if part.contains('-') {
            let range: Vec<&str> = part.split('-').collect();
            if range.len() == 2 {
                let start: usize = range[0].parse().context("잘못된 범위")?;
                let end: usize = range[1].parse().context("잘못된 범위")?;
                for i in start..=end.min(max) {
                    if i > 0 && i <= max {
                        indices.push(i - 1);
                    }
                }
            }
        } else if let Ok(n) = part.parse::<usize>() {
            if n > 0 && n <= max {
                indices.push(n - 1);
            }
        }
    }
    Ok(indices)
}

pub fn apply_selection(modules: &[String]) -> Result<()> {
    let current = read_blacklist_modules();
    let to_add: Vec<&str> = modules
        .iter()
        .map(|s| s.as_str())
        .filter(|m| !current.contains(&m.to_string()))
        .collect();

    let to_remove: Vec<String> = current
        .iter()
        .filter(|m| !modules.iter().any(|s| s == *m))
        .cloned()
        .collect();

    for module in &to_remove {
        remove_entry(module)?;
        println!("  🔓 블랙리스트 해제: {}", module);
    }

    if !to_add.is_empty() {
        let mut content = if std::path::Path::new(CONF_PATH).exists() {
            fs::read_to_string(CONF_PATH).unwrap_or_default()
        } else {
            String::new()
        };

        for module in &to_add {
            let entry = format!("blacklist {}", module);
            if !content.lines().any(|l| l.trim() == entry) {
                if !content.ends_with('\n') && !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(&entry);
                content.push('\n');
                println!("  🔒 블랙리스트 추가: {}", module);
            }
        }

        fs::write(CONF_PATH, content).with_context(|| format!("쓰기 실패: {CONF_PATH}"))?;
    }

    if to_add.is_empty() && to_remove.is_empty() {
        println!("변경 사항이 없습니다.");
    }

    Ok(())
}

pub fn apply_recommended() -> Result<Vec<String>> {
    let entries = scan_loaded_modules()?;
    let recommended: Vec<String> = entries
        .iter()
        .filter(|e| e.recommended)
        .map(|e| e.module.clone())
        .collect();

    if recommended.is_empty() {
        println!("권장 블랙리스트에 추가할 모듈이 없습니다.");
        return Ok(vec![]);
    }

    println!("권장 모듈 블랙리스트 적용:");
    for name in &recommended {
        println!("  ★ {}", name);
    }

    apply_selection(&recommended)?;
    Ok(recommended)
}

pub fn show_status() -> Result<()> {
    let entries = scan_loaded_modules()?;

    let blacklisted: Vec<&BlacklistEntry> =
        entries.iter().filter(|e| e.currently_blacklisted).collect();
    let recommended: Vec<&BlacklistEntry> = entries
        .iter()
        .filter(|e| e.recommended && !e.currently_blacklisted)
        .collect();

    println!("\n=== 블랙리스트 상태 ===\n");

    if !blacklisted.is_empty() {
        println!("현재 차단된 모듈 ({}개):", blacklisted.len());
        for e in &blacklisted {
            println!("  🔒 {:<20} [{}] {}", e.module, e.category, e.reason);
        }
    }

    if !recommended.is_empty() {
        println!("\n차단 권장 모듈 ({}개):", recommended.len());
        for e in recommended {
            println!("  ★  {:<20} [{}] {}", e.module, e.category, e.reason);
        }
    }

    println!("\n총 로드된 모듈: {}개", entries.len());
    println!("총 차단된 모듈: {}개", blacklisted.len());

    Ok(())
}

fn read_blacklist_modules() -> Vec<String> {
    if !std::path::Path::new(CONF_PATH).exists() {
        return vec![];
    }
    let content = fs::read_to_string(CONF_PATH).unwrap_or_default();
    content
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#') && t.starts_with("blacklist ")
        })
        .filter_map(|l| {
            let module = l.trim_start_matches("blacklist ").trim();
            if module.is_empty() {
                None
            } else {
                Some(module.to_string())
            }
        })
        .collect()
}

fn remove_entry(module: &str) -> Result<()> {
    if !std::path::Path::new(CONF_PATH).exists() {
        return Ok(());
    }
    let content = fs::read_to_string(CONF_PATH).unwrap_or_default();
    let entry = format!("blacklist {module}");
    let new_content: String = content
        .lines()
        .filter(|l| l.trim() != entry)
        .collect::<Vec<_>>()
        .join("\n");

    if new_content.trim().is_empty() {
        fs::remove_file(CONF_PATH)?;
    } else {
        fs::write(CONF_PATH, format!("{new_content}\n"))?;
    }
    Ok(())
}
