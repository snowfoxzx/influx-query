# influx-query

Single-binary CLI for querying InfluxDB v1 and v2.

## Distribution

This repository is designed to publish release binaries for:

- Linux: `x86_64`, `aarch64`
- macOS: `x86_64`, `aarch64`
- Windows: `x86_64`, `aarch64`

GitHub Actions builds tagged releases and uploads archives to GitHub Releases.
The bundled skill installs the correct binary for the current platform from the latest release by default.
Each release also publishes a `SHA256SUMS` file, and the install script verifies the downloaded archive before extraction.

## Build

```bash
cargo build --release
```

Binary path:

```bash
target/release/influx-query
```

## Release

Push a tag such as `v0.1.0` to trigger the release workflow:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow in [.github/workflows/release.yml](/Users/zhangxiao/Developer/rust/influx-query/.github/workflows/release.yml) uploads these archive names:

- `influx-query-linux-x86_64.tar.gz`
- `influx-query-linux-aarch64.tar.gz`
- `influx-query-macos-x86_64.tar.gz`
- `influx-query-macos-aarch64.tar.gz`
- `influx-query-windows-x86_64.zip`
- `influx-query-windows-aarch64.zip`
- `SHA256SUMS`

## Skill Distribution

The distributable skill lives in [skills/influx-query/SKILL.md](/Users/zhangxiao/Developer/rust/influx-query/skills/influx-query/SKILL.md).

The repo root also includes a minimal `.codex-plugin/plugin.json` manifest for Codex plugin packaging.
Claude Code can use the same skill directory when it is copied or installed into `~/.claude/skills`.

The skill instructions are written to be portable. Resolve `SKILL_DIR` to the directory that contains `SKILL.md`, then run:

```bash
sh "$SKILL_DIR/scripts/install_influx_query.sh"
```

Override the release source when needed:

```bash
INFLUX_QUERY_REPO=OWNER/REPO sh "$SKILL_DIR/scripts/install_influx_query.sh"
INFLUX_QUERY_VERSION=v0.1.0 sh "$SKILL_DIR/scripts/install_influx_query.sh"
```

## Examples

InfluxDB v1 with basic auth:

```bash
influx-query \
  --api v1 \
  --url http://localhost:8086 \
  --db metrics \
  --query 'select * from cpu limit 5' \
  --username alice \
  --password secret \
  --output table
```

InfluxDB v2 with token auth:

```bash
influx-query \
  --api v2 \
  --url https://influx.example.com \
  --org acme \
  --query 'from(bucket:"prod") |> range(start: -1h)' \
  --token "$INFLUX_TOKEN" \
  --output csv
```

## Output

- `table`: pretty-printed columns
- `json`: JSON array of records
- `csv`: CSV records with header
- `raw`: raw response body from the server

## Debugging

Add `--debug` to print the request method, URL, headers, response status, and raw response body to `stderr`.
Authorization headers are redacted in debug output.
