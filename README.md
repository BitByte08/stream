# stream-cli

**Raspberry Pi 4B + Ubuntu 24.04 Server 전용 스트리밍 최적화 CLI**

4K 60fps 영상 스트리밍을 위한 GPU 활성화, 드라이버/서비스 관리, mpv 최적화 파라미터 적용을 한 번에 처리합니다.

---

## 기능

| 기능 | 설명 |
|------|------|
| **GPU 활성화** | `vc4-kms-v3d` dtoverlay, GPU 메모리, CMA 설정 |
| **드라이버 블랙리스트** | 불필요한 커널 모듈 선택적으로 차단 (WiFi, BT, 등) |
| **서비스 관리** | systemd 서비스 비활성화로 CPU 절약 |
| **mpv 래퍼** | 4K 60fps 최적화 파라미터 자동 적용 |
| **종속성 관리** | mpv, ffmpeg 등 필수 패키지 apt 설치 |
| **H.265 변환** | H.264 → H.265 (RPi4 GPU HW 디코딩 지원) |
| **자동 트랜스코드 재생** | H.264 파일 재생 시 실시간 변환 |

---

## 설치

### 방법 1: 설치 스크립트 (권장)

```bash
curl -sL https://raw.githubusercontent.com/BitByte08/stream/main/install.sh | sudo bash
```

### 방법 2: deb 패키지 직접 설치 (최신 버전 자동)

```bash
wget "$(curl -sL https://api.github.com/repos/BitByte08/stream/releases/latest | grep 'browser_download_url.*arm64.deb' | head -1 | cut -d'"' -f4)" -O stream-cli.deb
sudo dpkg -i stream-cli.deb
sudo apt-get install -f -y
```

### 방법 3: 소스 빌드

```bash
curl -sL https://raw.githubusercontent.com/BitByte08/stream/main/install.sh | sudo bash -s -- --source
```

---

## 빠른 시작

```bash
# 전체 최적화 (부팅 시 자동 실행됨)
sudo stream-cli optimize

# 상태 확인
stream-cli status

# 재부팅
sudo reboot
```

---

## CLI 명령어

### GPU

```bash
sudo stream-cli gpu activate --gpu-mem 256    # GPU 활성화
sudo stream-cli gpu deactivate                # GPU 비활성화
stream-cli gpu status                         # GPU 상태 확인
```

### 드라이버/모듈

```bash
sudo stream-cli driver blacklist              # 인터랙티브 선택
sudo stream-cli driver recommended            # 권장 모듈 블랙리스트
stream-cli driver status                      # 현재 상태
sudo stream-cli driver load <module>          # 모듈 로드
sudo stream-cli driver unload <module>        # 모듈 언로드
```

### 서비스

```bash
sudo stream-cli service manage                # 인터랙티브 선택
sudo stream-cli service recommended           # 권장 서비스 비활성화
stream-cli service status                     # 서비스 상태
sudo stream-cli service disable <name>        # 서비스 비활성화
sudo stream-cli service enable <name>         # 서비스 활성화
```

### 종속성

```bash
sudo stream-cli dep install                   # 필수 패키지 설치
sudo stream-cli dep install-all               # 전체 설치
stream-cli dep status                         # 종속성 상태
stream-cli dep list                           # 패키지 목록
```

### 스트리밍

```bash
stream-cli stream play <file>                 # H.264 자동 감지→실시간 변환
stream-cli stream play <url>                  # 기본 (4K 60fps)
stream-cli stream play <url> --profile 4k30   # 4K 30fps
stream-cli stream play <url> --profile 1080p60
stream-cli stream play <url> --profile low-latency
stream-cli stream profiles                    # 프로필 목록
```

### 비디오 변환

```bash
stream-cli transcode convert <input>          # H.264 → H.265 변환
stream-cli transcode convert <input> --output <output>
stream-cli transcode batch <directory>        # 디렉토리 일괄 변환
stream-cli transcode batch <dir> --recursive  # 하위 디렉토리 포함
stream-cli transcode watch <directory>        # 디렉토리 감시 자동 변환
stream-cli transcode probe <file>             # 비디오 정보 조회
```

---

## 부팅 시 자동 최적화

`stream-cli-optimize.service`가 설치 시 자동 활성화됩니다:

```
[Unit]
After=local-fs.target
Before=multi-user.target
Before=bluetooth.service

[Service]
ExecStart=/usr/bin/stream-cli optimize

[Install]
WantedBy=sysinit.target
```

---

## RPi4 GPU 하드웨어 디코딩

| 코덱 | HW 디코딩 | 권장 |
|------|----------|------|
| H.265 (HEVC) | ✅ 지원 | GPU 디코딩 사용 |
| H.264 (AVC) | ❌ 미지원 | H.265 변환 후 재생 |
| VP9 | ❌ 미지원 | H.265 변환 권장 |

---

## 프로젝트 구조

```
src/
├── main.rs          # CLI (clap)
├── lib.rs           # 라이브러리 루트
├── gpu/
│   ├── mod.rs       # GPU 활성화/상태
│   └── config.rs    # vc4-kms-v3d 설정
├── driver/
│   ├── mod.rs       # 모듈 로드/언로드
│   └── blacklist.rs # 블랙리스트 관리
├── service/
│   └── mod.rs       # systemd 서비스 관리
├── stream/
│   ├── mod.rs       # mpv 래퍼 + H.264 자동 변환
│   └── params.rs    # 최적화 파라미터
├── dep/
│   └── mod.rs       # apt 종속성 관리
└── transcode/
    └── mod.rs       # H.264→H.265 변환
```

---

## 빌드

```bash
cargo build --release --target aarch64-unknown-linux-gnu
cargo deb --target aarch64-unknown-linux-gnu
```

---

## 릴리즈

태그 push 시 GitHub Actions가 자동으로 ARM64 `.deb` 빌드:

```bash
git tag v0.1.0
git push origin v0.1.0
```

---

## 라이선스

MIT