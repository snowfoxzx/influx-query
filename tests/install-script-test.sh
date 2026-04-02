#!/usr/bin/env sh

set -eu

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
SCRIPT="$ROOT_DIR/skills/influx-query/scripts/install_influx_query.sh"

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

test_checksum_url_for_tagged_release() {
  actual=$(
    INFLUX_QUERY_REPO=acme/influx-query \
    INFLUX_QUERY_VERSION=v1.2.3 \
    INFLUX_QUERY_OS=Linux \
    INFLUX_QUERY_ARCH=x86_64 \
    sh "$SCRIPT" print-checksum-url
  )

  assert_eq \
    "https://github.com/acme/influx-query/releases/download/v1.2.3/SHA256SUMS" \
    "$actual" \
    "checksum URL for tagged release"
}

test_extract_expected_checksum() {
  sums_file=$(mktemp)
  trap 'rm -f "$sums_file"' EXIT INT TERM
  cat >"$sums_file" <<'EOF'
aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  influx-query-linux-x86_64.tar.gz
bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  influx-query-macos-aarch64.tar.gz
EOF

  actual=$(
    INFLUX_QUERY_OS=Darwin \
    INFLUX_QUERY_ARCH=arm64 \
    INFLUX_QUERY_SHA256SUMS_FILE="$sums_file" \
    sh "$SCRIPT" print-expected-checksum
  )

  assert_eq \
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" \
    "$actual" \
    "extract matching checksum from SHA256SUMS"
}

test_unsupported_platform_fails() {
  if INFLUX_QUERY_OS=FreeBSD INFLUX_QUERY_ARCH=x86_64 sh "$SCRIPT" print-asset-name >/dev/null 2>&1; then
    printf 'assertion failed: unsupported platform should fail\n' >&2
    exit 1
  fi
}

test_skill_markdown_is_portable() {
  if rg -n 'skills/influx-query/' "$ROOT_DIR/skills/influx-query/SKILL.md" >/dev/null 2>&1; then
    printf 'assertion failed: SKILL.md should not hardcode repository-relative skill paths\n' >&2
    exit 1
  fi
}

test_codex_plugin_manifest_exists() {
  manifest="$ROOT_DIR/.codex-plugin/plugin.json"

  if [ ! -f "$manifest" ]; then
    printf 'assertion failed: codex plugin manifest should exist\n' >&2
    exit 1
  fi

  if ! rg -n '"skills":\s*"\./skills/"' "$manifest" >/dev/null 2>&1; then
    printf 'assertion failed: codex plugin manifest should expose ./skills/\n' >&2
    exit 1
  fi
}

test_latest_linux_x86_64
test_tagged_macos_arm64
test_windows_arm64_asset_name
test_checksum_url_for_tagged_release
test_extract_expected_checksum
test_unsupported_platform_fails
test_skill_markdown_is_portable
test_codex_plugin_manifest_exists

printf 'install-script tests passed\n'
