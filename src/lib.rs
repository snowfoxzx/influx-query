use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use clap::{Parser, ValueEnum};
use clap::error::ErrorKind;
use csv::{ReaderBuilder, WriterBuilder};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::{Value, json};
use std::fmt::Write as _;
use url::form_urlencoded::Serializer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiVersion {
    V1,
    V2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
    Csv,
    Raw,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryOptions {
    pub api_version: ApiVersion,
    pub base_url: String,
    pub query: String,
    pub database: Option<String>,
    pub retention_policy: Option<String>,
    pub org: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub output: OutputFormat,
    pub debug: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltRequest {
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ApiArg {
    V1,
    V2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OutputArg {
    Table,
    Json,
    Csv,
    Raw,
}

#[derive(Debug, Parser)]
#[command(name = "influx-query", version, about = "Query InfluxDB v1 and v2 from a single binary")]
struct Cli {
    #[arg(long = "api", value_enum)]
    api: ApiArg,
    #[arg(long = "url")]
    url: String,
    #[arg(long = "query")]
    query: String,
    #[arg(long = "db")]
    database: Option<String>,
    #[arg(long = "rp")]
    retention_policy: Option<String>,
    #[arg(long = "org")]
    org: Option<String>,
    #[arg(long = "token")]
    token: Option<String>,
    #[arg(long = "username")]
    username: Option<String>,
    #[arg(long = "password")]
    password: Option<String>,
    #[arg(long = "output", value_enum, default_value = "table")]
    output: OutputArg,
    #[arg(long = "debug")]
    debug: bool,
}

pub fn parse_args<I>(args: I) -> Result<QueryOptions>
where
    I: IntoIterator<Item = String>,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                print!("{err}");
                std::process::exit(0);
            }
            _ => return Err(anyhow!(err.to_string())),
        },
    };

    let api_version = match cli.api {
        ApiArg::V1 => ApiVersion::V1,
        ApiArg::V2 => ApiVersion::V2,
    };

    let output = match cli.output {
        OutputArg::Table => OutputFormat::Table,
        OutputArg::Json => OutputFormat::Json,
        OutputArg::Csv => OutputFormat::Csv,
        OutputArg::Raw => OutputFormat::Raw,
    };

    match api_version {
        ApiVersion::V1 if cli.database.is_none() => bail!("--db is required for --api v1"),
        ApiVersion::V2 if cli.org.is_none() => bail!("--org is required for --api v2"),
        _ => {}
    }

    Ok(QueryOptions {
        api_version,
        base_url: cli.url,
        query: cli.query,
        database: cli.database,
        retention_policy: cli.retention_policy,
        org: cli.org,
        token: cli.token,
        username: cli.username,
        password: cli.password,
        output,
        debug: cli.debug,
    })
}

pub fn build_request(options: &QueryOptions) -> Result<BuiltRequest> {
    match options.api_version {
        ApiVersion::V1 => build_v1_request(options),
        ApiVersion::V2 => build_v2_request(options),
    }
}

pub fn render_table(columns: &[String], rows: &[Vec<String>]) -> String {
    let widths = column_widths(columns, rows);
    let mut out = String::new();

    push_table_row(&mut out, columns, &widths);
    out.push('\n');

    let separators = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>();
    push_table_row(&mut out, &separators, &widths);

    for row in rows {
        out.push('\n');
        push_table_row(&mut out, row, &widths);
    }

    out
}

pub fn execute_query(options: &QueryOptions) -> Result<String> {
    let request = build_request(options)?;
    if options.debug {
        eprintln!("{}", format_debug_request(&request));
    }
    let client = Client::builder().build().context("failed to build HTTP client")?;
    let mut req = match request.method {
        "POST" => client.post(&request.url),
        "GET" => client.get(&request.url),
        method => bail!("unsupported method {method}"),
    };

    let mut headers = HeaderMap::new();
    for (name, value) in &request.headers {
        headers.insert(
            HeaderName::from_bytes(name.as_bytes()).context("invalid header name")?,
            HeaderValue::from_str(value).context("invalid header value")?,
        );
    }

    req = req.headers(headers);
    if let Some(body) = request.body {
        req = req.body(body);
    }

    let response = req.send().context("request failed")?;
    let status = response.status();
    let body = response.text().context("failed to read response body")?;
    if options.debug {
        eprintln!("{}", format_debug_response(status.as_u16(), &body));
    }
    if !status.is_success() {
        bail!("request failed with status {status}: {body}");
    }

    Ok(body)
}

pub fn format_response(options: &QueryOptions, body: &str) -> Result<String> {
    match options.output {
        OutputFormat::Raw => Ok(body.to_string()),
        _ => match options.api_version {
            ApiVersion::V1 => format_v1_response(options.output.clone(), body),
            ApiVersion::V2 => format_v2_response(options.output.clone(), body),
        },
    }
}

fn build_v1_request(options: &QueryOptions) -> Result<BuiltRequest> {
    let mut form = Serializer::new(String::new());
    form.append_pair(
        "db",
        options
            .database
            .as_deref()
            .ok_or_else(|| anyhow!("--db is required for --api v1"))?,
    );

    if let Some(retention_policy) = options.retention_policy.as_deref() {
        form.append_pair("rp", retention_policy);
    }

    form.append_pair("q", &options.query);

    let mut headers = vec![(
        "Content-Type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    )];
    maybe_push_auth_headers(options, &mut headers)?;

    Ok(BuiltRequest {
        method: "POST",
        url: format!("{}/query", trim_base_url(&options.base_url)),
        headers,
        body: Some(form.finish()),
    })
}

fn build_v2_request(options: &QueryOptions) -> Result<BuiltRequest> {
    let org = options
        .org
        .as_deref()
        .ok_or_else(|| anyhow!("--org is required for --api v2"))?;
    let mut headers = vec![
        ("Content-Type".to_string(), "application/json".to_string()),
        (
            "Accept".to_string(),
            match options.output {
                OutputFormat::Json => "application/json".to_string(),
                _ => "application/csv".to_string(),
            },
        ),
    ];
    maybe_push_auth_headers(options, &mut headers)?;

    Ok(BuiltRequest {
        method: "POST",
        url: format!(
            "{}/api/v2/query?org={}",
            trim_base_url(&options.base_url),
            percent_encode(org)
        ),
        headers,
        body: Some(json!({ "query": options.query }).to_string()),
    })
}

fn maybe_push_auth_headers(options: &QueryOptions, headers: &mut Vec<(String, String)>) -> Result<()> {
    if let Some(token) = options.token.as_deref() {
        headers.push(("Authorization".to_string(), format!("Token {token}")));
        return Ok(());
    }

    if let (Some(username), Some(password)) = (
        options.username.as_deref(),
        options.password.as_deref(),
    ) {
        let encoded = base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
        headers.push(("Authorization".to_string(), format!("Basic {encoded}")));
    }

    Ok(())
}

fn trim_base_url(base_url: &str) -> String {
    base_url.trim_end_matches('/').to_string()
}

fn percent_encode(value: &str) -> String {
    let mut serializer = Serializer::new(String::new());
    serializer.append_pair("v", value);
    serializer.finish()["v=".len()..].to_string()
}

fn column_widths(columns: &[String], rows: &[Vec<String>]) -> Vec<usize> {
    let mut widths = columns.iter().map(|column| column.len()).collect::<Vec<_>>();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(index) {
                *width = (*width).max(cell.len());
            }
        }
    }
    widths
}

fn push_table_row(out: &mut String, cells: &[String], widths: &[usize]) {
    for (index, cell) in cells.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        let width = widths.get(index).copied().unwrap_or(cell.len());
        let _ = write!(out, "{cell:<width$}");
    }
}

fn format_v1_response(output: OutputFormat, body: &str) -> Result<String> {
    let response: V1Response = serde_json::from_str(body).context("failed to parse v1 JSON response")?;
    let Some(results) = response.results else {
        return Ok(no_rows_message(output));
    };
    let series = results
        .into_iter()
        .flat_map(|result| result.series.unwrap_or_default())
        .collect::<Vec<_>>();
    if series.is_empty() {
        return Ok(no_rows_message(output));
    }

    let columns = series[0].columns.clone();
    let rows = series
        .iter()
        .flat_map(|series| series.values.clone().unwrap_or_default())
        .map(|row| row.into_iter().map(json_value_to_string).collect::<Vec<_>>())
        .collect::<Vec<_>>();

    format_records(output, &columns, &rows)
}

fn format_v2_response(output: OutputFormat, body: &str) -> Result<String> {
    let cleaned = body
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    if cleaned.trim().is_empty() {
        return Ok(no_rows_message(output));
    }

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(cleaned.as_bytes());
    let headers = reader
        .headers()
        .context("failed to parse CSV headers")?
        .iter()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.context("failed to parse CSV record")?;
        rows.push(record.iter().map(ToOwned::to_owned).collect::<Vec<_>>());
    }

    format_records(output, &headers, &rows)
}

fn no_rows_message(output: OutputFormat) -> String {
    match output {
        OutputFormat::Json => "[]".to_string(),
        OutputFormat::Csv => String::new(),
        OutputFormat::Table | OutputFormat::Raw => "(no rows)".to_string(),
    }
}

fn format_debug_request(request: &BuiltRequest) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "[debug] request {} {}", request.method, request.url);
    for (name, value) in &request.headers {
        let rendered = if name.eq_ignore_ascii_case("Authorization") {
            redact_auth_header(value)
        } else {
            value.clone()
        };
        let _ = writeln!(out, "[debug] header {name}: {rendered}");
    }
    if let Some(body) = &request.body {
        let _ = writeln!(out, "[debug] body {body}");
    }
    out.trim_end().to_string()
}

fn format_debug_response(status: u16, body: &str) -> String {
    format!("[debug] response status {status}\n[debug] response body {body}")
}

fn redact_auth_header(value: &str) -> String {
    let mut parts = value.splitn(2, ' ');
    match (parts.next(), parts.next()) {
        (Some(scheme), Some(_)) => format!("{scheme} <redacted>"),
        _ => "<redacted>".to_string(),
    }
}

fn format_records(output: OutputFormat, columns: &[String], rows: &[Vec<String>]) -> Result<String> {
    match output {
        OutputFormat::Table => Ok(render_table(columns, rows)),
        OutputFormat::Csv => {
            let mut writer = WriterBuilder::new().from_writer(Vec::new());
            writer.write_record(columns).context("failed to write CSV header")?;
            for row in rows {
                writer.write_record(row).context("failed to write CSV row")?;
            }
            let bytes = writer.into_inner().context("failed to finalize CSV writer")?;
            String::from_utf8(bytes).context("failed to encode CSV as UTF-8")
        }
        OutputFormat::Json => {
            let values = rows
                .iter()
                .map(|row| {
                    let mut object = serde_json::Map::new();
                    for (column, cell) in columns.iter().zip(row.iter()) {
                        object.insert(column.clone(), Value::String(cell.clone()));
                    }
                    Value::Object(object)
                })
                .collect::<Vec<_>>();
            serde_json::to_string_pretty(&values).context("failed to encode JSON output")
        }
        OutputFormat::Raw => Ok(String::new()),
    }
}

fn json_value_to_string(value: Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value,
        other => other.to_string(),
    }
}

#[derive(Debug, Deserialize)]
struct V1Response {
    results: Option<Vec<V1Result>>,
}

#[derive(Debug, Deserialize)]
struct V1Result {
    series: Option<Vec<V1Series>>,
}

#[derive(Debug, Deserialize, Clone)]
struct V1Series {
    columns: Vec<String>,
    values: Option<Vec<Vec<Value>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_v1_args_with_basic_auth() {
        let args = vec![
            "influx-query".to_string(),
            "--api".to_string(),
            "v1".to_string(),
            "--url".to_string(),
            "http://localhost:8086".to_string(),
            "--db".to_string(),
            "metrics".to_string(),
            "--query".to_string(),
            "select * from cpu".to_string(),
            "--username".to_string(),
            "alice".to_string(),
            "--password".to_string(),
            "secret".to_string(),
            "--output".to_string(),
            "json".to_string(),
            "--debug".to_string(),
        ];

        let parsed = parse_args(args).expect("args should parse");

        assert_eq!(parsed.api_version, ApiVersion::V1);
        assert_eq!(parsed.database.as_deref(), Some("metrics"));
        assert_eq!(parsed.username.as_deref(), Some("alice"));
        assert_eq!(parsed.password.as_deref(), Some("secret"));
        assert_eq!(parsed.output, OutputFormat::Json);
        assert!(parsed.debug);
    }

    #[test]
    fn builds_v1_request_with_query_string_and_basic_auth() {
        let options = QueryOptions {
            api_version: ApiVersion::V1,
            base_url: "http://localhost:8086/".to_string(),
            query: "select * from cpu".to_string(),
            database: Some("metrics".to_string()),
            retention_policy: Some("autogen".to_string()),
            org: None,
            token: None,
            username: Some("alice".to_string()),
            password: Some("secret".to_string()),
            output: OutputFormat::Json,
            debug: false,
        };

        let request = build_request(&options).expect("request should build");

        assert_eq!(request.method, "POST");
        assert_eq!(request.url, "http://localhost:8086/query");
        assert!(request
            .body
            .as_deref()
            .is_some_and(|body| body.contains("db=metrics")));
        assert!(request
            .body
            .as_deref()
            .is_some_and(|body| body.contains("rp=autogen")));
        assert!(request
            .body
            .as_deref()
            .is_some_and(|body| body.contains("q=select+*+from+cpu")));
        assert!(request.headers.iter().any(|(name, value)| {
            name == "Authorization" && value.starts_with("Basic ")
        }));
    }

    #[test]
    fn builds_v2_request_with_flux_and_token_auth() {
        let options = QueryOptions {
            api_version: ApiVersion::V2,
            base_url: "https://influx.example.com".to_string(),
            query: "from(bucket:\"prod\") |> range(start: -1h)".to_string(),
            database: None,
            retention_policy: None,
            org: Some("acme".to_string()),
            token: Some("token-123".to_string()),
            username: None,
            password: None,
            output: OutputFormat::Csv,
            debug: false,
        };

        let request = build_request(&options).expect("request should build");

        assert_eq!(request.method, "POST");
        assert_eq!(request.url, "https://influx.example.com/api/v2/query?org=acme");
        assert!(request.headers.iter().any(|(name, value)| {
            name == "Authorization" && value == "Token token-123"
        }));
        assert!(request.headers.iter().any(|(name, value)| {
            name == "Accept" && value == "application/csv"
        }));
        assert_eq!(
            request.body.as_deref(),
            Some("{\"query\":\"from(bucket:\\\"prod\\\") |> range(start: -1h)\"}")
        );
    }

    #[test]
    fn renders_table_with_header_and_rows() {
        let rendered = render_table(
            &["name".to_string(), "value".to_string()],
            &[
                vec!["cpu0".to_string(), "42".to_string()],
                vec!["cpu1".to_string(), "43".to_string()],
            ],
        );

        assert_eq!(rendered, "name value\n---- -----\ncpu0 42   \ncpu1 43   ");
    }

    #[test]
    fn v1_empty_results_are_not_silent() {
        let rendered = format_response(
            &QueryOptions {
                api_version: ApiVersion::V1,
                base_url: "http://localhost:8086".to_string(),
                query: "select * from cpu".to_string(),
                database: Some("metrics".to_string()),
                retention_policy: None,
                org: None,
                token: None,
                username: None,
                password: None,
                output: OutputFormat::Table,
                debug: false,
            },
            r#"{"results":[{"statement_id":0}]}"#,
        )
        .expect("formatting should succeed");

        assert_eq!(rendered, "(no rows)");
    }

    #[test]
    fn formats_debug_output_with_redacted_auth() {
        let rendered = format_debug_request(&BuiltRequest {
            method: "POST",
            url: "http://localhost:8086/query".to_string(),
            headers: vec![
                ("Authorization".to_string(), "Basic abc123".to_string()),
                ("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string()),
            ],
            body: Some("db=metrics&q=select+1".to_string()),
        });

        assert!(rendered.contains("[debug] request POST http://localhost:8086/query"));
        assert!(rendered.contains("[debug] header Authorization: Basic <redacted>"));
        assert!(rendered.contains("[debug] body db=metrics&q=select+1"));
    }
}
