#!/usr/bin/env sh

set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
SCRIPT="$ROOT_DIR/skills/influxdb-query/scripts/install_influx_query.sh"

assert_eq() {
  expected=$1
  actual=$2
  message=$3

  if [ "$expected" != "$actual" ]; then
    printf 'assertion failed: %s\nexpected: %s\nactual:   %s\n' "$message" "$expected" "$actual" >&2
    exit 1
  fi
}

test_latest_linux_x86_64() {
  actual=$(
    INFLUX_QUERY_REPO=acme/influx-query \
    INFLUX_QUERY_OS=Linux \
    INFLUX_QUERY_ARCH=x86_64 \
    sh "$SCRIPT" print-download-url
  )

  assert_eq \
    "https://github.com/acme/influx-query/releases/latest/download/influx-query-linux-x86_64.tar.gz" \
    "$actual" \
    "linux x86_64 latest URL"
}

test_tagged_macos_arm64() {
  actual=$(
    INFLUX_QUERY_REPO=acme/influx-query \
    INFLUX_QUERY_VERSION=v1.2.3 \
    INFLUX_QUERY_OS=Darwin \
    INFLUX_QUERY_ARCH=arm64 \
    sh "$SCRIPT" print-download-url
  )

  assert_eq \
    "https://github.com/acme/influx-query/releases/download/v1.2.3/influx-query-macos-aarch64.tar.gz" \
    "$actual" \
    "macOS arm64 tagged URL"
}

test_windows_arm64_asset_name() {
  actual=$(
    INFLUX_QUERY_OS=MINGW64_NT-10.0 \
    INFLUX_QUERY_ARCH=aarch64 \
    sh "$SCRIPT" print-asset-name
  )

  assert_eq \
    "influx-query-windows-aarch64.zip" \
    "$actual" \
    "windows arm64 asset name"
}

test_unsupported_platform_fails() {
  if INFLUX_QUERY_OS=FreeBSD INFLUX_QUERY_ARCH=x86_64 sh "$SCRIPT" print-asset-name >/dev/null 2>&1; then
    printf 'assertion failed: unsupported platform should fail\n' >&2
    exit 1
  fi
}

test_latest_linux_x86_64
test_tagged_macos_arm64
test_windows_arm64_asset_name
test_unsupported_platform_fails

printf 'install-script tests passed\n'
