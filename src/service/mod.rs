use anyhow::{Context, Result};
use std::io::{self, BufRead, Write};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub description: String,
    pub state: ServiceState,
    pub recommended_disable: bool,
    pub category: String,
    pub reason: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ServiceState {
    Running,
    Stopped,
    Disabled,
    Enabled,
    Masked,
    Unknown,
}

const STREAMING_DISABLE_SERVICES: &[(&str, &str, &str)] = &[
    ("bluetooth.service", "Bluetooth", "스트리밍 시 불필요"),
    ("hciuart.service", "Bluetooth UART", "스트리밍 시 불필요"),
    (
        "rpibluetooth.service",
        "RPi Bluetooth",
        "스트리밍 시 불필요",
    ),
    ("wpa_supplicant.service", "WiFi 연결", "유선 사용 시 불필요"),
    (
        "dhcpcd.service",
        "DHCP Client",
        "정적 IP 사용 시 비활성화 가능",
    ),
    ("avahi-daemon.service", "mDNS/Bonjour", "스트리밍 시 불필요"),
    ("cron.service", "Cron 작업", "스크린케스트 시 비활성화 가능"),
    (
        "rsyslog.service",
        "로그 수집",
        "성능 최적화 시 비활성화 가능",
    ),
    ("triggerhappy.service", "GPIO 이벤트", "스트리밍 시 불필요"),
    (
        "alsa-restore.service",
        "ALSA 설정",
        "HDMI 오디오 사용 시 비활성화 가능",
    ),
    (
        "alsa-state.service",
        "ALSA 상태",
        "HDMI 오디오 사용 시 비활성화 가능",
    ),
    (
        "rpi-display-backlight.service",
        "디스플레이 백라이트",
        "고정 백라이트 설정 시",
    ),
];

const OPTIONAL_DISABLE_SERVICES: &[(&str, &str, &str)] = &[
    ("nginx.service", "Web Server", "서버 운영 시 필요"),
    ("apache2.service", "Web Server", "서버 운영 시 필요"),
    ("ssh.service", "SSH 서버", "원격 관리 필요 시 유지"),
    (
        "systemd-journald.service",
        "Journal 로그",
        "디버깅 필요 시 유지",
    ),
];

pub fn scan_services() -> Result<Vec<ServiceInfo>> {
    let output = std::process::Command::new("systemctl")
        .args([
            "list-units",
            "--type=service",
            "--all",
            "--no-pager",
            "--plain",
        ])
        .output()
        .context("systemctl 실행 실패")?;

    let content = String::from_utf8_lossy(&output.stdout);
    let mut services = Vec::new();

    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let name = parts[0];
        if !name.ends_with(".service") {
            continue;
        }

        let load_state = parts[1];
        let active_state = parts[2];
        let sub_state = parts[3];

        let state = determine_state(load_state, active_state, sub_state);

        let (category, reason, recommended) = get_service_info(name);

        if category.is_empty() {
            continue;
        }

        let description = get_service_description(name)?;

        services.push(ServiceInfo {
            name: name.to_string(),
            description,
            state,
            recommended_disable: recommended,
            category,
            reason,
        });
    }

    services.sort_by(|a, b| {
        b.recommended_disable
            .cmp(&a.recommended_disable)
            .then(a.category.cmp(&b.category))
            .then(a.name.cmp(&b.name))
    });

    Ok(services)
}

fn determine_state(load: &str, active: &str, _sub: &str) -> ServiceState {
    if load == "masked" {
        return ServiceState::Masked;
    }
    match active {
        "active" => ServiceState::Running,
        "inactive" | "failed" => ServiceState::Stopped,
        _ => ServiceState::Unknown,
    }
}

fn get_service_info(name: &str) -> (String, String, bool) {
    for (svc, cat, reason) in STREAMING_DISABLE_SERVICES {
        if name == *svc {
            return ((*cat).into(), (*reason).into(), true);
        }
    }
    for (svc, cat, reason) in OPTIONAL_DISABLE_SERVICES {
        if name == *svc {
            return ((*cat).into(), (*reason).into(), false);
        }
    }
    if name.contains("bluetooth") {
        return ("Bluetooth".into(), "스트리밍 시 비활성화 권장".into(), true);
    }
    if name.contains("wifi") || name.contains("wpa") {
        return ("WiFi".into(), "유선 사용 시 비활성화 권장".into(), true);
    }
    if name.contains("alsa") || name.contains("sound") {
        return ("Audio".into(), "HDMI 오디오 사용 시".into(), false);
    }
    ("".into(), "".into(), false)
}

fn get_service_description(name: &str) -> Result<String> {
    let output = std::process::Command::new("systemctl")
        .args(["show", name, "--property=Description", "--no-pager"])
        .output();

    match output {
        Ok(o) => {
            let desc = String::from_utf8_lossy(&o.stdout);
            let desc = desc
                .lines()
                .next()
                .and_then(|l| l.strip_prefix("Description="))
                .map(|s| s.to_string())
                .unwrap_or_default();
            Ok(desc)
        }
        _ => Ok(String::new()),
    }
}

pub fn interactive_manage() -> Result<Vec<String>> {
    let services = scan_services()?;

    if services.is_empty() {
        println!("검색된 서비스가 없습니다.");
        return Ok(vec![]);
    }

    let mut selected: Vec<bool> = services
        .iter()
        .map(|s| {
            s.state == ServiceState::Stopped
                || s.state == ServiceState::Disabled
                || s.state == ServiceState::Masked
        })
        .collect();

    loop {
        println!("\n╔══════════════════════════════════════════════════════════════════╗");
        println!("║              Systemd 서비스 관리 (스트리밍 최적화)              ║");
        println!("╚══════════════════════════════════════════════════════════════════╝");
        println!("  [■] 비활성화됨  [□] 활성화됨  ★ 비활성화 권장");
        println!("  명령: 번호, 범위(1-5), r(권장), a(전체), n(활성화), d(적용), q(종료)\n");

        println!(
            "{:<4} {:<3} {:<5} {:<30} {:<12} {:<10} {}",
            "", "", "상태", "서비스명", "카테고리", "현재", "비고"
        );
        println!("{}", "─".repeat(100));

        for (i, svc) in services.iter().enumerate() {
            let check = if selected[i] { "■" } else { "□" };
            let rec = if svc.recommended_disable { "★" } else { " " };
            let state_icon = match svc.state {
                ServiceState::Running => "🟢",
                ServiceState::Stopped => "🔴",
                ServiceState::Disabled => "⚫",
                ServiceState::Masked => "🔒",
                ServiceState::Unknown => "⚪",
                ServiceState::Enabled => "🔵",
            };

            println!(
                "{:<3} {} {:1}{:<1} {:<30} {:<12} {:<10} {}",
                i + 1,
                check,
                rec,
                "",
                svc.name,
                svc.category,
                state_icon,
                svc.reason
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
                for (i, svc) in services.iter().enumerate() {
                    selected[i] = svc.recommended_disable;
                }
            }
            "a" => selected.iter_mut().for_each(|s| *s = true),
            "n" => selected.iter_mut().for_each(|s| *s = false),
            _ => {
                if let Ok(indices) = parse_selection(&input, services.len()) {
                    for idx in indices {
                        selected[idx] = !selected[idx];
                    }
                }
            }
        }
    }

    let to_disable: Vec<String> = services
        .iter()
        .enumerate()
        .filter(|(i, s)| selected[*i] && s.state == ServiceState::Running)
        .map(|(_, s)| s.name.clone())
        .collect();

    let to_enable: Vec<String> = services
        .iter()
        .enumerate()
        .filter(|(i, s)| !selected[*i] && s.state != ServiceState::Running)
        .map(|(_, s)| s.name.clone())
        .collect();

    println!("\n변경 예정:");
    if !to_disable.is_empty() {
        println!("  비활성화:");
        for name in &to_disable {
            println!("    🔴 {}", name);
        }
    }
    if !to_enable.is_empty() {
        println!("  활성화:");
        for name in &to_enable {
            println!("    🟢 {}", name);
        }
    }

    Ok(to_disable)
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

pub fn disable_service(name: &str) -> Result<()> {
    std::process::Command::new("systemctl")
        .args(["stop", name])
        .status()
        .with_context(|| format!("{} 정지 실패", name))?;

    std::process::Command::new("systemctl")
        .args(["disable", name])
        .status()
        .with_context(|| format!("{} 비활성화 실패", name))?;

    println!("  🔴 {} 비활성화 완료", name);
    Ok(())
}

pub fn enable_service(name: &str) -> Result<()> {
    std::process::Command::new("systemctl")
        .args(["enable", name])
        .status()
        .with_context(|| format!("{} 활성화 실패", name))?;

    std::process::Command::new("systemctl")
        .args(["start", name])
        .status()
        .with_context(|| format!("{} 시작 실패", name))?;

    println!("  🟢 {} 활성화 완료", name);
    Ok(())
}

pub fn apply_selection(services_to_disable: &[String]) -> Result<()> {
    for name in services_to_disable {
        disable_service(name)?;
    }
    Ok(())
}

pub fn apply_recommended() -> Result<Vec<String>> {
    let services = scan_services()?;
    let to_disable: Vec<String> = services
        .iter()
        .filter(|s| s.recommended_disable && s.state == ServiceState::Running)
        .map(|s| s.name.clone())
        .collect();

    if to_disable.is_empty() {
        println!("비활성화 권장 서비스가 없습니다.");
        return Ok(vec![]);
    }

    println!("권장 서비스 비활성화:");
    for name in &to_disable {
        println!("  ★ {}", name);
    }

    apply_selection(&to_disable)?;
    Ok(to_disable)
}

pub fn show_status() -> Result<()> {
    let services = scan_services()?;

    let disabled: Vec<&ServiceInfo> = services
        .iter()
        .filter(|s| s.state == ServiceState::Stopped || s.state == ServiceState::Disabled)
        .collect();

    let running_recommended_disable: Vec<&ServiceInfo> = services
        .iter()
        .filter(|s| s.recommended_disable && s.state == ServiceState::Running)
        .collect();

    println!("\n=== 서비스 상태 ===\n");

    if !running_recommended_disable.is_empty() {
        println!("비활성화 권장 ({}개):", running_recommended_disable.len());
        for s in running_recommended_disable {
            println!("  🟢 {:<30} [{}] {}", s.name, s.category, s.reason);
        }
    }

    if !disabled.is_empty() {
        println!("\n현재 비활성화됨 ({}개):", disabled.len());
        for s in &disabled {
            println!("  🔴 {:<30} [{}] {}", s.name, s.category, s.reason);
        }
    }

    println!("\n총 서비스: {}개", services.len());
    println!("비활성화됨: {}개", disabled.len());

    Ok(())
}
