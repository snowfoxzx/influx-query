#!/usr/bin/env sh

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
SKILL_DIR=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
BIN_DIR="$SKILL_DIR/bin"
REPO="${INFLUX_QUERY_REPO:-zhangxiao/influx-query}"
VERSION="${INFLUX_QUERY_VERSION:-latest}"

detect_os() {
  if [ -n "${INFLUX_QUERY_OS:-}" ]; then
    printf '%s\n' "$INFLUX_QUERY_OS"
    return 0
  fi

  uname -s
}

detect_arch() {
  if [ -n "${INFLUX_QUERY_ARCH:-}" ]; then
    printf '%s\n' "$INFLUX_QUERY_ARCH"
    return 0
  fi

  uname -m
}

normalized_os() {
  case "$(detect_os)" in
    Linux)
      printf 'linux\n'
      ;;
    Darwin)
      printf 'macos\n'
      ;;
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
      printf 'windows\n'
      ;;
    *)
      printf 'unsupported operating system: %s\n' "$(detect_os)" >&2
      exit 1
      ;;
  esac
}

normalized_arch() {
  case "$(detect_arch)" in
    x86_64|amd64)
      printf 'x86_64\n'
      ;;
    arm64|aarch64)
      printf 'aarch64\n'
      ;;
    *)
      printf 'unsupported architecture: %s\n' "$(detect_arch)" >&2
      exit 1
      ;;
  esac
}

archive_extension() {
  os=$(normalized_os)
  case "$os" in
    windows)
      printf 'zip\n'
      ;;
    *)
      printf 'tar.gz\n'
      ;;
  esac
}

asset_name() {
  os=$(normalized_os)
  arch=$(normalized_arch)
  ext=$(archive_extension)
  printf 'influx-query-%s-%s.%s\n' "$os" "$arch" "$ext"
}

download_url() {
  asset=$(asset_name)
  if [ "$VERSION" = "latest" ]; then
    printf 'https://github.com/%s/releases/latest/download/%s\n' "$REPO" "$asset"
  else
    printf 'https://github.com/%s/releases/download/%s/%s\n' "$REPO" "$VERSION" "$asset"
  fi
}

download_archive() {
  url=$(download_url)
  archive_path=$1

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$archive_path"
    return 0
  fi

  if command -v wget >/dev/null 2>&1; then
    wget -qO "$archive_path" "$url"
    return 0
  fi

  printf 'curl or wget is required to download %s\n' "$url" >&2
  exit 1
}

extract_archive() {
  archive_path=$1
  temp_dir=$2

  case "$(archive_extension)" in
    tar.gz)
      tar -xzf "$archive_path" -C "$temp_dir"
      ;;
    zip)
      if command -v unzip >/dev/null 2>&1; then
        unzip -oq "$archive_path" -d "$temp_dir"
      elif command -v powershell >/dev/null 2>&1; then
        powershell -NoProfile -Command "Expand-Archive -LiteralPath '$archive_path' -DestinationPath '$temp_dir' -Force"
      else
        printf 'unzip or powershell is required to extract %s\n' "$archive_path" >&2
        exit 1
      fi
      ;;
  esac
}

binary_name() {
  os=$(normalized_os)
  case "$os" in
    windows)
      printf 'influx-query.exe\n'
      ;;
    *)
      printf 'influx-query\n'
      ;;
  esac
}

install_binary() {
  temp_dir=$(mktemp -d)
  trap 'rm -rf "$temp_dir"' EXIT INT TERM

  archive_path="$temp_dir/$(asset_name)"
  download_archive "$archive_path"
  extract_archive "$archive_path" "$temp_dir"

  mkdir -p "$BIN_DIR"
  binary=$(binary_name)
  cp "$temp_dir/$binary" "$BIN_DIR/$binary"

  os=$(normalized_os)
  case "$os" in
    windows)
      ;;
    *)
      chmod +x "$BIN_DIR/$binary"
      ;;
  esac

  printf 'installed %s\n' "$BIN_DIR/$binary"
}

case "${1:-install}" in
  install)
    install_binary
    ;;
  print-asset-name)
    asset_name
    ;;
  print-download-url)
    download_url
    ;;
  *)
    printf 'usage: %s [install|print-asset-name|print-download-url]\n' "$0" >&2
    exit 1
    ;;
esac
