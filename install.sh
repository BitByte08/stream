#!/bin/bash
set -e

REPO_OWNER="BitByte08"
REPO_NAME="stream-cli"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}[INFO]${NC} $1"; }
ok()    { echo -e "${GREEN}[OK]${NC} $1"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
err()   { echo -e "${RED}[ERROR]${NC} $1"; }

check_arch() {
    local arch=$(uname -m)
    if [ "$arch" != "aarch64" ] && [ "$arch" != "arm64" ]; then
        err "이 스크립트는 ARM64 (Raspberry Pi 4) 전용입니다. 현재: $arch"
        exit 1
    fi
}

check_os() {
    if [ ! -f /etc/os-release ]; then
        err "OS 감지 실패"
        exit 1
    fi
    source /etc/os-release
    if [ "$ID" != "ubuntu" ] && [ "$ID" != "raspbian" ] && [ "$ID" != "debian" ]; then
        warn "Ubuntu/Debian 계열이 아닐 수 있습니다: $ID"
    fi
    info "OS: $PRETTY_NAME"
}

get_latest_tag() {
    if command -v curl &>/dev/null; then
        curl -sL "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" \
            | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/'
    elif command -v wget &>/dev/null; then
        wget -qO- "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" \
            | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/'
    else
        err "curl 또는 wget 필요"
        exit 1
    fi
}

download_and_install() {
    local tag=$1
    local deb_name="stream-cli_${tag#v}_arm64.deb"
    local url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${tag}/${deb_name}"

    info "다운로드: ${url}"
    local tmpdir=$(mktemp -d)
    local deb_path="${tmpdir}/${deb_name}"

    if command -v curl &>/dev/null; then
        curl -sL "$url" -o "$deb_path"
    else
        wget -q "$url" -O "$deb_path"
    fi

    if [ ! -f "$deb_path" ] || [ ! -s "$deb_path" ]; then
        err "다운로드 실패. 릴리즈를 확인하세요:"
        echo "  https://github.com/${REPO_OWNER}/${REPO_NAME}/releases"
        exit 1
    fi

    info "시스템 패키지 복구..."
    sudo dpkg --configure -a
    sudo apt-get install -f -y

    info "패키지 설치..."
    sudo dpkg -i "$deb_path" || {
        warn "의존성 해결 중..."
        sudo apt-get install -f -y || {
            err "패키지 의존성 문제. 수동 설치:"
            echo "  sudo dpkg --configure -a"
            echo "  sudo apt-get install -f -y"
            echo "  sudo dpkg -i ${deb_path}"
            exit 1
        }
        sudo dpkg -i "$deb_path"
    }

    rm -rf "$tmpdir"
    ok "설치 완료!"
}

install_from_source() {
    info "소스에서 빌드..."
    sudo apt-get update
    sudo apt-get install -y build-essential pkg-config libssl-dev

    if ! command -v cargo &>/dev/null; then
        info "Rust 설치..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi

    if [ ! -d "${REPO_NAME}" ]; then
        git clone "https://github.com/${REPO_OWNER}/${REPO_NAME}.git"
    fi
    cd "${REPO_NAME}"

    cargo build --release
    sudo cp target/release/stream-cli /usr/bin/stream-cli
    sudo cp debian/stream-cli-optimize.service /etc/systemd/system/
    sudo systemctl daemon-reload
    sudo systemctl enable stream-cli-optimize.service

    ok "빌드 설치 완료!"
}

usage() {
    echo ""
    echo "  stream-cli 설치 스크립트"
    echo "  Raspberry Pi 4B + Ubuntu 24.04 Server 전용"
    echo ""
    echo "  사용법:"
    echo "    curl -sL <url>/install.sh | sudo bash"
    echo "    curl -sL <url>/install.sh | sudo bash -s -- --source"
    echo ""
    echo "  옵션:"
    echo "    --source   deb 대신 소스에서 빌드"
    echo "    --help     도움말"
    echo ""
}

main() {
    local from_source=false
    if [ "$1" = "--source" ]; then
        from_source=true
    elif [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
        usage
        exit 0
    fi

    echo ""
    echo "╔══════════════════════════════════════════╗"
    echo "║     stream-cli for Raspberry Pi 4B       ║"
    echo "║     4K 60fps 스트리밍 최적화              ║"
    echo "╚══════════════════════════════════════════╝"
    echo ""

    check_arch
    check_os

    if [ "$from_source" = true ]; then
        install_from_source
    else
        local tag=$(get_latest_tag)
        if [ -z "$tag" ]; then
            warn "릴리즈를 찾을 수 없습니다. 소스 빌드로 전환..."
            install_from_source
        else
            info "최신 버전: ${tag}"
            download_and_install "$tag"
        fi
    fi

    echo ""
    echo "╔══════════════════════════════════════════╗"
    echo "║  설치 완료! 다음 명령으로 시작:          ║"
    echo "║                                          ║"
    echo "║  sudo stream-cli optimize                ║"
    echo "║  stream-cli status                       ║"
    echo "╚══════════════════════════════════════════╝"
    echo ""
}

main "$@"
