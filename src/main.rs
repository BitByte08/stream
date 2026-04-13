use clap::{Parser, Subcommand};
use stream_cli::{
    dep::{check_dependencies, install_all, install_missing, optional_packages, required_packages},
    driver::{
        blacklist::{
            apply_recommended, apply_selection, interactive_select, scan_loaded_modules,
            show_status as blacklist_status,
        },
        load_module, unload_module,
    },
    gpu::{activate, apply_cmdline_tweaks, config::GpuConfig, deactivate, detect as gpu_detect},
    service::{
        apply_recommended as service_recommended, apply_selection as service_apply,
        disable_service, enable_service, interactive_manage, show_status as service_status,
    },
    stream::{detect_environment, play_with_profile},
    transcode::{probe, transcode, transcode_batch, watch_and_transcode, TranscodeConfig},
};

#[derive(Parser)]
#[command(name = "stream-cli")]
#[command(bin_name = "stream-cli")]
#[command(version, about = "Raspberry Pi 4B 스트리밍 최적화 CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "GPU 설정")]
    Gpu {
        #[command(subcommand)]
        subcmd: GpuCommands,
    },
    #[command(about = "드라이버/모듈 관리")]
    Driver {
        #[command(subcommand)]
        subcmd: DriverCommands,
    },
    #[command(about = "서비스 관리")]
    Service {
        #[command(subcommand)]
        subcmd: ServiceCommands,
    },
    #[command(about = "종속성 관리")]
    Dep {
        #[command(subcommand)]
        subcmd: DepCommands,
    },
    #[command(about = "영상 스트리밍")]
    Stream {
        #[command(subcommand)]
        subcmd: StreamCommands,
    },
    #[command(about = "비디오 트랜스코드 (H.264→H.265)")]
    Transcode {
        #[command(subcommand)]
        subcmd: TranscodeCommands,
    },
    #[command(about = "시스템 상태 조회")]
    Status,
    #[command(about = "권장 설정 한 번에 적용")]
    Optimize,
}

#[derive(Subcommand)]
enum GpuCommands {
    #[command(about = "GPU 상태 확인")]
    Status,
    #[command(about = "GPU 활성화")]
    Activate {
        #[arg(long, default_value_t = 256)]
        gpu_mem: u32,
    },
    #[command(about = "GPU 비활성화")]
    Deactivate,
    #[command(about = "cmdline 파라미터 적용")]
    Cmdline,
    #[command(about = "GPU 설정 조회")]
    Config,
}

#[derive(Subcommand)]
enum DriverCommands {
    #[command(about = "모듈 블랙리스트 관리 (인터랙티브)")]
    Blacklist,
    #[command(about = "블랙리스트 상태")]
    Status,
    #[command(about = "권장 블랙리스트 적용")]
    Recommended,
    #[command(about = "모듈 로드")]
    Load { module: String },
    #[command(about = "모듈 언로드")]
    Unload { module: String },
}

#[derive(Subcommand)]
enum ServiceCommands {
    #[command(about = "서비스 관리 (인터랙티브)")]
    Manage,
    #[command(about = "서비스 상태")]
    Status,
    #[command(about = "권장 서비스 비활성화")]
    Recommended,
    #[command(about = "서비스 비활성화")]
    Disable { name: String },
    #[command(about = "서비스 활성화")]
    Enable { name: String },
}

#[derive(Subcommand)]
enum DepCommands {
    #[command(about = "종속성 상태 확인")]
    Status,
    #[command(about = "필수 종속성 설치")]
    Install,
    #[command(about = "모든 종속성 설치")]
    InstallAll,
    #[command(about = "패키지 목록")]
    List,
}

#[derive(Subcommand)]
enum StreamCommands {
    #[command(about = "영상 재생")]
    Play {
        #[arg(required = true)]
        source: String,
        #[arg(long, default_value = "4k60")]
        profile: String,
        #[arg(long)]
        extra: Vec<String>,
    },
    #[command(about = "스트리밍 환경 확인")]
    Status,
    #[command(about = "프로필 목록")]
    Profiles,
}

#[derive(Subcommand)]
enum TranscodeCommands {
    #[command(about = "H.264→H.265 변환")]
    Convert {
        #[arg(required = true)]
        input: String,
        #[arg(long)]
        output: Option<String>,
        #[arg(long, default_value_t = 23)]
        crf: u8,
        #[arg(long)]
        sw: bool,
    },
    #[command(about = "디렉토리 일괄 변환")]
    Batch {
        #[arg(required = true)]
        directory: String,
        #[arg(long)]
        recursive: bool,
        #[arg(long, default_value_t = 23)]
        crf: u8,
    },
    #[command(about = "디렉토리 감시 자동 변환")]
    Watch {
        #[arg(required = true)]
        directory: String,
        #[arg(long, default_value_t = 23)]
        crf: u8,
    },
    #[command(about = "비디오 파일 정보 조회")]
    Probe {
        #[arg(required = true)]
        input: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Gpu { subcmd } => handle_gpu(subcmd)?,
        Commands::Driver { subcmd } => handle_driver(subcmd)?,
        Commands::Service { subcmd } => handle_service(subcmd)?,
        Commands::Dep { subcmd } => handle_dep(subcmd)?,
        Commands::Stream { subcmd } => handle_stream(subcmd)?,
        Commands::Transcode { subcmd } => handle_transcode(subcmd)?,
        Commands::Status => show_full_status()?,
        Commands::Optimize => run_optimize()?,
    }
    Ok(())
}

fn handle_gpu(cmd: GpuCommands) -> anyhow::Result<()> {
    match cmd {
        GpuCommands::Status => {
            let status = gpu_detect()?;
            println!("\n=== GPU 상태 ===\n");
            println!(
                "  드라이버 로드: {}",
                if status.driver_loaded { "✅" } else { "❌" }
            );
            println!(
                "  dtoverlay 설정: {}",
                if status.dtoverlay_set { "✅" } else { "❌" }
            );
            println!("  GPU 메모리: {} MB", status.gpu_mem_mb.unwrap_or(0));
            println!(
                "  FW 라이브러리: {}",
                if status.firmware_exists { "✅" } else { "❌" }
            );
            if !status.driver_loaded || !status.dtoverlay_set {
                println!("\n  권장: sudo stream-cli gpu activate");
            }
        }
        GpuCommands::Activate { gpu_mem } => {
            println!("GPU 활성화 중... (gpu_mem={}MB)", gpu_mem);
            activate(gpu_mem)?;
            apply_cmdline_tweaks()?;
            println!("\n✅ GPU 활성화 완료. 재부팅 필요: sudo reboot");
        }
        GpuCommands::Deactivate => {
            println!("GPU 비활성화 중...");
            deactivate()?;
            println!("\n✅ GPU 비활성화 완료. 재부팅 필요: sudo reboot");
        }
        GpuCommands::Cmdline => {
            apply_cmdline_tweaks()?;
            println!("✅ cmdline 파라미터 적용 완료.");
        }
        GpuCommands::Config => {
            let config = GpuConfig::new_4k60();
            println!("\n=== GPU 설정 (4K 60fps) ===\n");
            println!("dtoverlay: {}", config.overlay);
            println!("gpu_mem: {} MB", config.gpu_mem);
            println!("cma: {} MB", config.cma_size_mb);
            println!(
                "HDMI: {}x{} @ {}Hz",
                config.hdmi_cvt.width, config.hdmi_cvt.height, config.hdmi_cvt.framerate
            );
        }
    }
    Ok(())
}

fn handle_driver(cmd: DriverCommands) -> anyhow::Result<()> {
    match cmd {
        DriverCommands::Blacklist => {
            let entries = scan_loaded_modules()?;
            let selected = interactive_select(&entries)?;
            if !selected.is_empty() {
                apply_selection(&selected)?;
                println!("\n✅ 블랙리스트 적용 완료. 재부팅 권장.");
            }
        }
        DriverCommands::Status => blacklist_status()?,
        DriverCommands::Recommended => {
            let applied = apply_recommended()?;
            if !applied.is_empty() {
                println!("\n✅ {}개 모듈 블랙리스트 적용.", applied.len());
            }
        }
        DriverCommands::Load { module } => {
            load_module(&module)?;
            println!("✅ {} 로드 완료.", module);
        }
        DriverCommands::Unload { module } => {
            unload_module(&module)?;
            println!("✅ {} 언로드 완료.", module);
        }
    }
    Ok(())
}

fn handle_service(cmd: ServiceCommands) -> anyhow::Result<()> {
    match cmd {
        ServiceCommands::Manage => {
            let to_disable = interactive_manage()?;
            if !to_disable.is_empty() {
                service_apply(&to_disable)?;
                println!("\n✅ 서비스 변경 완료.");
            }
        }
        ServiceCommands::Status => service_status()?,
        ServiceCommands::Recommended => {
            let applied = service_recommended()?;
            if !applied.is_empty() {
                println!("\n✅ {}개 서비스 비활성화.", applied.len());
            }
        }
        ServiceCommands::Disable { name } => disable_service(&name)?,
        ServiceCommands::Enable { name } => enable_service(&name)?,
    }
    Ok(())
}

fn handle_dep(cmd: DepCommands) -> anyhow::Result<()> {
    match cmd {
        DepCommands::Status => {
            let deps = check_dependencies()?;
            println!("\n=== 종속성 상태 ===\n");
            for d in &deps {
                let icon = if d.installed { "✅" } else { "❌" };
                let ver = d.version.as_deref().unwrap_or("-");
                println!("  {} {:<25} {}", icon, d.name, ver);
            }
            let missing = deps.iter().filter(|d| !d.installed).count();
            if missing > 0 {
                println!("\n  {}개 누락. 설치: sudo stream-cli dep install", missing);
            }
        }
        DepCommands::Install => install_missing()?,
        DepCommands::InstallAll => install_all()?,
        DepCommands::List => {
            println!("\n필수 패키지:");
            for p in required_packages() {
                println!("  - {}", p);
            }
            println!("\n선택 패키지:");
            for p in optional_packages() {
                println!("  - {}", p);
            }
        }
    }
    Ok(())
}

fn handle_stream(cmd: StreamCommands) -> anyhow::Result<()> {
    match cmd {
        StreamCommands::Play {
            source,
            profile,
            extra,
        } => {
            println!("재생: {} (프로필: {})", source, profile);
            play_with_profile(&source, &profile, &extra)?;
        }
        StreamCommands::Status => {
            let status = detect_environment()?;
            println!("\n=== 스트리밍 환경 ===\n");
            println!("  mpv: {}", if status.mpv_installed { "✅" } else { "❌" });
            if let Some(ver) = &status.mpv_version {
                println!("       {}", ver);
            }
            println!(
                "  ffmpeg: {}",
                if status.ffmpeg_installed {
                    "✅"
                } else {
                    "❌"
                }
            );
        }
        StreamCommands::Profiles => {
            println!("\n=== 스트리밍 프로필 ===\n");
            for (name, desc) in [
                ("4k60", "4K 60fps - 최고 화질"),
                ("4k30", "4K 30fps - 화질 우선"),
                ("1080p60", "1080p 60fps - 성능 우선"),
                ("low-latency", "저지연 - 실시간 스트리밍"),
            ] {
                println!("  {:<15} {}", name, desc);
            }
        }
    }
    Ok(())
}

fn show_full_status() -> anyhow::Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║           Raspberry Pi 스트리밍 시스템 상태              ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let gpu = gpu_detect()?;
    println!("GPU:");
    println!(
        "  드라이버: {} | dtoverlay: {} | 메모리: {}MB",
        if gpu.driver_loaded { "✅" } else { "❌" },
        if gpu.dtoverlay_set { "✅" } else { "❌" },
        gpu.gpu_mem_mb.unwrap_or(0)
    );

    let stream = detect_environment()?;
    println!("\n스트리밍:");
    println!(
        "  mpv: {} | ffmpeg: {}",
        if stream.mpv_installed { "✅" } else { "❌" },
        if stream.ffmpeg_installed {
            "✅"
        } else {
            "❌"
        }
    );

    let deps = check_dependencies()?;
    let missing = deps.iter().filter(|d| !d.installed).count();
    println!("\n종속성: {}개 누락", missing);

    blacklist_status()?;
    service_status()?;
    Ok(())
}

fn handle_transcode(cmd: TranscodeCommands) -> anyhow::Result<()> {
    match cmd {
        TranscodeCommands::Convert {
            input,
            output,
            crf,
            sw,
        } => {
            let mut config =
                TranscodeConfig::new_h264_to_h265(&input, output.as_deref()).with_crf(crf);
            if sw {
                config = config.with_software_fallback();
            }
            transcode(&config)?;
        }
        TranscodeCommands::Batch {
            directory,
            recursive,
            crf,
        } => {
            let result = transcode_batch(&directory, recursive, crf)?;
            if result.failed > 0 {
                println!("\n오류:");
                for e in &result.errors {
                    println!("  ❌ {}", e);
                }
            }
        }
        TranscodeCommands::Watch { directory, crf } => {
            watch_and_transcode(&directory, crf)?;
        }
        TranscodeCommands::Probe { input } => {
            let info = probe(&input)?;
            println!("\n=== 비디오 정보 ===\n");
            println!("  파일: {}", info.path);
            if let Some(codec) = &info.video_codec {
                println!("  비디오 코덱: {}", codec);
            }
            if let (Some(w), Some(h)) = (info.width, info.height) {
                println!("  해상도: {}x{}", w, h);
            }
            if let Some(fps) = info.fps {
                println!("  FPS: {:.2}", fps);
            }
            if let Some(codec) = &info.audio_codec {
                println!("  오디오 코덱: {}", codec);
            }
            if let Some(dur) = info.duration_secs {
                println!("  길이: {:.1}초", dur);
            }
            println!("  크기: {} bytes", info.size_bytes);
            if info.needs_transcode {
                println!("\n  ⚠ H.264 감지 → H.265 변환 권장");
                println!("  sudo stream-cli transcode convert \"{}\"", input);
            } else {
                println!("\n  ✅ GPU 디코딩 가능 (변환 불필요)");
            }
        }
    }
    Ok(())
}

fn run_optimize() -> anyhow::Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║              스트리밍 최적화 - 전체 적용                 ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    println!("[1/5] 종속성 확인...");
    install_missing()?;

    println!("\n[2/5] GPU 활성화...");
    activate(256)?;
    apply_cmdline_tweaks()?;
    println!("✅ GPU 설정 완료");

    println!("\n[3/5] 드라이버 블랙리스트 적용...");
    let mods = apply_recommended()?;
    println!("✅ {}개 모듈 블랙리스트 적용", mods.len());

    println!("\n[4/5] 서비스 비활성화...");
    let svcs = service_recommended()?;
    println!("✅ {}개 서비스 비활성화", svcs.len());

    println!("\n[5/5] 완료");
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  ✅ 모든 최적화 완료! 재부팅 필요: sudo reboot           ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
    Ok(())
}
