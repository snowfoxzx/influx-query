---
name: influxdb-query
description: Use when querying InfluxDB v1 or v2 directly, especially for ad hoc data inspection, validating InfluxQL or Flux queries, or debugging request and response details with a local CLI.
---

# InfluxDB Query

Use this skill to run direct InfluxDB queries through the bundled `influx-query` CLI.

## When To Use

- You need to query InfluxDB v1 with InfluxQL.
- You need to query InfluxDB v2 with Flux.
- You want a single CLI that works across macOS, Linux, and Windows.
- You need `--debug` output to inspect request and response details.

## Workflow

1. Ensure the binary is installed:

```bash
sh skills/influxdb-query/scripts/install_influx_query.sh
```

2. Run a query with the installed binary:

```bash
skills/influxdb-query/bin/influx-query --help
```

3. Use one of these templates.

InfluxDB v1:

```bash
skills/influxdb-query/bin/influx-query \
  --api v1 \
  --url http://HOST:8086 \
  --db DATABASE \
  --query 'select * from measurement limit 5'
```

InfluxDB v2:

```bash
skills/influxdb-query/bin/influx-query \
  --api v2 \
  --url https://HOST \
  --org ORG \
  --query 'from(bucket:"BUCKET") |> range(start: -1h)' \
  --token "$INFLUX_TOKEN"
```

## Debugging

- Add `--debug` to print request method, URL, headers, response status, and raw response body to `stderr`.
- Use `--output raw` when you need the exact server response on `stdout`.

## Updating

- Re-run the install script to fetch the latest GitHub Release binary.
- Set `INFLUX_QUERY_VERSION=vX.Y.Z` before running the install script to pin a specific release.

